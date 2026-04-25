//! Spawns `claude -p` and drives a single turn.

use std::collections::HashMap;
use std::time::Duration;

use base64::Engine as _;
use tokio::sync::mpsc;

use crate::cancel::CancelToken;
use crate::error::Error;
use crate::event::AgentEvent;
use crate::request::{Attachment, ToolPolicy, TurnOutcome, TurnRequest};
use crate::shell_env::SHELL_ENV;

use super::protocol::{ProtocolMsg, parse_protocol_line};

/// How long a subprocess spawn will wait for the background shell-env
/// harvest before giving up and inheriting the parent env. Long-running
/// turns can afford a few hundred milliseconds on the first spawn; by the
/// time a second turn runs the harvest is cached.
const SHELL_ENV_TIMEOUT: Duration = Duration::from_millis(500);

/// Run a single prompt turn by spawning `claude -p` and streaming its output.
/// Returns the session ID from the result message (for `--resume` on next
/// turn).
///
/// Uses `std::process` with a background reader thread (not tokio) because
/// Iced's async runtime configuration has historically made `tokio::process`
/// brittle; shelling out + an std thread + a tokio channel is portable.
pub async fn run_turn(
    req: TurnRequest,
    events: mpsc::Sender<AgentEvent>,
    cancel: CancelToken,
) -> Result<TurnOutcome, Error> {
    use std::io::Write;

    let content_blocks = assemble_user_content(&req.prompt, &req.attachments);
    let stream_msg = serde_json::json!({
        "type": "user",
        "message": { "role": "user", "content": content_blocks },
    });
    let stream_line = serde_json::to_string(&stream_msg)
        .map_err(|e| Error::Process(format!("failed to encode stream-json input: {e}")))?;

    let mut cmd = std::process::Command::new("claude");
    cmd.arg("-p")
        .arg("--input-format")
        .arg("stream-json")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--include-partial-messages")
        .current_dir(&req.working_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());

    if matches!(req.tools, ToolPolicy::BypassAll) {
        cmd.arg("--permission-mode").arg("bypassPermissions");
    }

    if let Some(sid) = req.session_id.as_deref() {
        cmd.arg("--resume").arg(sid);
    }

    if let Some(model) = req.model.as_deref() {
        cmd.arg("--model").arg(model);
    }

    if let Some(system) = join_system_additions(&req.system_additions) {
        cmd.arg("--append-system-prompt").arg(system);
    }

    SHELL_ENV.apply(&mut cmd, SHELL_ENV_TIMEOUT);

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::Spawn(format!("failed to spawn claude: {e}")))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stream_line.as_bytes()).ok();
        stdin.write_all(b"\n").ok();
        // stdin drops here, closing the pipe.
    }

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::Process("no stdout from claude subprocess".into()))?;

    // Read stdout lines in a background thread, forward via channel.
    let (line_tx, mut line_rx) = mpsc::unbounded_channel::<Option<String>>();

    std::thread::spawn(move || {
        use std::io::BufRead;
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if line_tx.send(Some(l)).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = line_tx.send(None); // EOF
    });

    let mut result_session_id = String::new();

    while let Some(data) = line_rx.recv().await {
        if cancel.is_cancelled() {
            tracing::info!("cancelling claude turn, killing child");
            let _ = child.kill();
            while line_rx.recv().await.is_some() {}
            return Err(Error::Cancelled);
        }

        let Some(line) = data else { break }; // EOF
        if line.is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<ProtocolMsg>(&line) else {
            continue;
        };

        if msg.type_ == "result" {
            tracing::debug!(result_line = %line, "claude result message");
            if let Some(sid) = &msg.session_id {
                result_session_id = sid.clone();
            }
            if msg.is_error == Some(true) {
                let error_msg = msg
                    .result
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error")
                    .to_string();
                return Err(Error::Process(error_msg));
            }
        }

        for event in parse_protocol_line(&msg) {
            if events.send(event).await.is_err() {
                // Receiver gone — abort.
                let _ = child.kill();
                return Err(Error::Cancelled);
            }
        }
    }

    std::thread::spawn(move || {
        child.wait().ok();
    });

    if result_session_id.is_empty() {
        Err(Error::Protocol("no session_id in claude result".into()))
    } else {
        Ok(TurnOutcome {
            session_id: result_session_id,
        })
    }
}

/// Join non-empty `system_additions` with blank-line separators, returning
/// `None` when nothing is contributed (so callers can skip the
/// `--append-system-prompt` flag entirely).
fn join_system_additions(additions: &[String]) -> Option<String> {
    let parts: Vec<&str> = additions.iter().map(String::as_str).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

/// Walk `prompt` and split it into wire-format content blocks. Markdown
/// links of the form `[label](attach:<id>)` whose `<id>` resolves in
/// `attachments` are replaced with image content blocks (for image media
/// types) or a plain-text fallback (for non-image attachments). Unresolved
/// or malformed spans are emitted as their original literal text so the
/// model still sees a useful link.
fn assemble_user_content(
    prompt: &str,
    attachments: &HashMap<String, Attachment>,
) -> Vec<serde_json::Value> {
    let mut blocks: Vec<serde_json::Value> = Vec::new();
    let mut cursor = 0usize;

    while let Some(open_rel) = prompt[cursor..].find('[') {
        let open = cursor + open_rel;
        let pre = &prompt[cursor..open];

        // Look for the matching `]` on the same line; bail to text on miss.
        let Some((label, link_start)) = find_label_end(prompt, open) else {
            append_text(&mut blocks, &prompt[cursor..=open]);
            cursor = open + 1;
            continue;
        };

        // Expect "(attach:" right after the label.
        let Some(id_start) = prompt[link_start..]
            .strip_prefix("(attach:")
            .map(|_| link_start + "(attach:".len())
        else {
            append_text(&mut blocks, &prompt[cursor..=open]);
            cursor = open + 1;
            continue;
        };

        // Id terminates at `)` on the same line.
        let Some(id_len) = prompt[id_start..].find([')', '\n']) else {
            append_text(&mut blocks, &prompt[cursor..=open]);
            cursor = open + 1;
            continue;
        };
        if prompt.as_bytes()[id_start + id_len] != b')' {
            append_text(&mut blocks, &prompt[cursor..=open]);
            cursor = open + 1;
            continue;
        }
        let id = &prompt[id_start..id_start + id_len];
        let span_end = id_start + id_len + 1;

        match attachments.get(id) {
            Some(att) if att.media_type.starts_with("image/") => {
                append_text(&mut blocks, pre);
                let b64 = base64::engine::general_purpose::STANDARD.encode(&att.bytes);
                blocks.push(serde_json::json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": att.media_type,
                        "data": b64,
                    }
                }));
                cursor = span_end;
            }
            Some(att) => {
                // Non-image: keep label visible, drop the bytes (model has no
                // way to consume them in a content block).
                append_text(&mut blocks, pre);
                append_text(
                    &mut blocks,
                    &format!("[attachment: {} ({} bytes)]", att.label, att.bytes.len()),
                );
                cursor = span_end;
                let _ = label;
            }
            None => {
                // Unresolved id — keep the link literal in the text.
                append_text(&mut blocks, &prompt[cursor..span_end]);
                cursor = span_end;
                let _ = label;
            }
        }
    }

    append_text(&mut blocks, &prompt[cursor..]);
    if blocks.is_empty() {
        blocks.push(serde_json::json!({"type": "text", "text": ""}));
    }
    blocks
}

/// Append `s` as a text content block, merging with the previous block when
/// it is also text. Empty strings are dropped.
fn append_text(blocks: &mut Vec<serde_json::Value>, s: &str) {
    if s.is_empty() {
        return;
    }
    if let Some(last) = blocks.last_mut()
        && last.get("type").and_then(|v| v.as_str()) == Some("text")
    {
        let prev = last["text"].as_str().unwrap_or("");
        let merged = format!("{prev}{s}");
        last["text"] = serde_json::Value::String(merged);
        return;
    }
    blocks.push(serde_json::json!({ "type": "text", "text": s }));
}

/// Returns `(label, position_after_closing_bracket)` if `prompt[open..]`
/// starts a markdown link label terminated by `]` before the next newline.
fn find_label_end(prompt: &str, open: usize) -> Option<(&str, usize)> {
    let after_open = open + 1;
    let rest = &prompt[after_open..];
    let close_rel = rest.find([']', '\n'])?;
    if rest.as_bytes()[close_rel] != b']' {
        return None;
    }
    let label = &rest[..close_rel];
    Some((label, after_open + close_rel + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(name: &str) -> Attachment {
        Attachment {
            label: name.to_string(),
            media_type: "image/png".to_string(),
            bytes: vec![1, 2, 3, 4],
        }
    }

    fn text_block(blocks: &[serde_json::Value], i: usize) -> &str {
        blocks[i]["text"].as_str().unwrap_or("")
    }

    #[test]
    fn plain_text_yields_single_text_block() {
        let blocks = assemble_user_content("hello world", &HashMap::new());
        assert_eq!(blocks.len(), 1);
        assert_eq!(text_block(&blocks, 0), "hello world");
    }

    #[test]
    fn single_image_link_emits_image_block() {
        let mut atts = HashMap::new();
        atts.insert("a1".to_string(), img("clip.png"));
        let blocks = assemble_user_content("look at [clip.png](attach:a1)!", &atts);
        assert_eq!(blocks.len(), 3);
        assert_eq!(text_block(&blocks, 0), "look at ");
        assert_eq!(blocks[1]["type"], "image");
        assert_eq!(blocks[1]["source"]["media_type"], "image/png");
        assert_eq!(text_block(&blocks, 2), "!");
    }

    #[test]
    fn two_links_interleaved_with_text() {
        let mut atts = HashMap::new();
        atts.insert("a".to_string(), img("a.png"));
        atts.insert("b".to_string(), img("b.png"));
        let blocks = assemble_user_content(
            "first [a.png](attach:a) then [b.png](attach:b) done",
            &atts,
        );
        assert_eq!(blocks.len(), 5);
        assert_eq!(text_block(&blocks, 0), "first ");
        assert_eq!(blocks[1]["type"], "image");
        assert_eq!(text_block(&blocks, 2), " then ");
        assert_eq!(blocks[3]["type"], "image");
        assert_eq!(text_block(&blocks, 4), " done");
    }

    #[test]
    fn unresolved_id_falls_through_to_text() {
        let blocks = assemble_user_content("see [thing](attach:missing)", &HashMap::new());
        assert_eq!(blocks.len(), 1);
        assert_eq!(text_block(&blocks, 0), "see [thing](attach:missing)");
    }

    #[test]
    fn unrelated_markdown_link_is_left_alone() {
        let blocks = assemble_user_content("see [docs](https://example.com)", &HashMap::new());
        assert_eq!(blocks.len(), 1);
        assert_eq!(text_block(&blocks, 0), "see [docs](https://example.com)");
    }

    #[test]
    fn malformed_link_falls_through() {
        // No closing paren on the same line → not a link.
        let mut atts = HashMap::new();
        atts.insert("a".to_string(), img("a.png"));
        let blocks = assemble_user_content("oops [a.png](attach:a\nrest", &atts);
        assert_eq!(blocks.len(), 1);
        assert_eq!(text_block(&blocks, 0), "oops [a.png](attach:a\nrest");
    }

    #[test]
    fn empty_prompt_yields_empty_text_block() {
        let blocks = assemble_user_content("", &HashMap::new());
        assert_eq!(blocks.len(), 1);
        assert_eq!(text_block(&blocks, 0), "");
    }

    #[test]
    fn join_system_additions_skips_empty_and_joins() {
        assert_eq!(join_system_additions(&[]), None);
        assert_eq!(
            join_system_additions(&[String::new(), "  ".to_string()]),
            Some("  ".to_string())
        );
        assert_eq!(
            join_system_additions(&["a".to_string(), "b".to_string()]),
            Some("a\n\nb".to_string())
        );
    }
}
