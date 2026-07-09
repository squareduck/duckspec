//! Spawns `claude -p` and drives a single turn.

use base64::Engine as _;
use tokio::sync::mpsc;

use crate::attach::{self, Segment};
use crate::cancel::CancelToken;
use crate::error::Error;
use crate::event::AgentEvent;
use crate::request::{ToolPolicy, TurnOutcome, TurnRequest};

use super::protocol::{ProtocolMsg, parse_protocol_line};
use super::spawn::claude_command;

/// Built-in CLI tools that can't function in our headless `-p` invocation —
/// either they need an interactive UI the CLI auto-denies (AskUserQuestion,
/// plan mode) or they depend on a parent harness duckboard doesn't provide
/// (cron, scheduling, remote control, push notifications, worktree sessions).
/// Letting the model attempt them just wastes a turn on a synthetic deny.
const DISALLOWED_TOOLS: &str = "AskUserQuestion,EnterPlanMode,ExitPlanMode,\
    CronCreate,CronDelete,CronList,ScheduleWakeup,RemoteTrigger,\
    PushNotification,EnterWorktree,ExitWorktree";

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

    let mut cmd = claude_command();
    cmd.arg("-p")
        .arg("--input-format")
        .arg("stream-json")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--include-partial-messages")
        .arg("--disallowedTools")
        .arg(DISALLOWED_TOOLS)
        // Auto-memory writes/reads MEMORY.md under ~/.claude/projects/… and
        // injects the contents into the system prompt. Useful in standalone
        // Claude Code but noisy inside duckboard, where the model is driven
        // turn-by-turn against a duckspec scope and shouldn't be steering
        // itself with stale per-project notes. Found by probe: this is the
        // single recognised settings key; unknown keys are silently dropped
        // in -p mode.
        .arg("--settings")
        .arg(r#"{"autoMemoryEnabled":false}"#)
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

/// Walk `prompt` for attach markers and encode Anthropic content blocks.
fn assemble_user_content(
    prompt: &str,
    attachments: &std::collections::HashMap<String, crate::request::Attachment>,
) -> Vec<serde_json::Value> {
    encode_anthropic(&attach::walk(prompt, attachments))
}

/// Encode neutral attach segments as Anthropic-style content blocks.
fn encode_anthropic(segments: &[Segment]) -> Vec<serde_json::Value> {
    let mut blocks: Vec<serde_json::Value> = Vec::new();
    for segment in segments {
        match segment {
            Segment::Text(text) => {
                blocks.push(serde_json::json!({ "type": "text", "text": text }));
            }
            Segment::Image { media_type, bytes } => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
                blocks.push(serde_json::json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": media_type,
                        "data": b64,
                    }
                }));
            }
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::Attachment;
    use std::collections::HashMap;

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
    fn anthropic_encode_uses_source_media_type() {
        let mut atts = HashMap::new();
        atts.insert("a1".to_string(), img("clip.png"));
        let blocks = assemble_user_content("look at [clip.png](attach:a1)!", &atts);
        assert_eq!(blocks.len(), 3);
        assert_eq!(text_block(&blocks, 0), "look at ");
        assert_eq!(blocks[1]["type"], "image");
        assert_eq!(blocks[1]["source"]["media_type"], "image/png");
        assert!(blocks[1]["source"]["data"].as_str().is_some());
        assert_eq!(text_block(&blocks, 2), "!");
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
