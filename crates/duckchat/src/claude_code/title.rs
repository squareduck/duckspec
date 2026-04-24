//! One-shot summariser that asks Claude Haiku for a short session title.
//!
//! Deliberately avoids `run_turn`: no session resume, no tools, no permission
//! prompts, no stream-json. Plain `claude -p --model ... "<prompt>"` with the
//! response read off stdout as text. The subprocess is short-lived — we shell
//! out on a background thread and deliver the result via a oneshot channel so
//! the async caller doesn't block the runtime.

use std::io::Read;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::sync::oneshot;

use crate::error::Error;
use crate::request::TitleRequest;
use crate::shell_env::SHELL_ENV;

use super::TITLE_MODEL;

/// Matches the run-turn budget — keeps both spawns consistent.
const SHELL_ENV_TIMEOUT: Duration = Duration::from_millis(500);

/// Max characters of the assistant reply we feed into the summariser. Keeps
/// the prompt tight when the agent produces a long opening response.
const ASSISTANT_REPLY_CHAR_CAP: usize = 500;

pub async fn title_summary(req: TitleRequest, working_dir: &Path) -> Result<String, Error> {
    let prompt = build_prompt(&req);
    let working_dir = working_dir.to_path_buf();

    let (tx, rx) = oneshot::channel();
    std::thread::spawn(move || {
        let result = run_sync(&prompt, &working_dir);
        let _ = tx.send(result);
    });

    match rx.await {
        Ok(result) => result,
        Err(_) => Err(Error::Other(
            "title summariser thread vanished without reply".into(),
        )),
    }
}

fn build_prompt(req: &TitleRequest) -> String {
    let truncated = truncate_chars(&req.assistant_reply, ASSISTANT_REPLY_CHAR_CAP);
    let mut out = String::from(
        "Write a 3-6 word title naming what the USER is trying to do in \
this session. Lean on the User message and any Hint lines. Use the \
assistant reply only to extract the topic or domain when the user message \
is brief (e.g. a bare slash command). Ignore the assistant's opening \
filler — phrases like \"Based on my review\", \"Here's what I found\", \
\"Let me look at\", \"After reviewing\" — and focus on the substantive \
thing being discussed. Plain text, no quotes, no trailing punctuation. \
Sentence case — capitalize only the first word and proper nouns.\n\n",
    );
    for hint in &req.context_hints {
        let trimmed = hint.trim();
        if trimmed.is_empty() {
            continue;
        }
        out.push_str("Hint: ");
        out.push_str(trimmed);
        out.push_str("\n\n");
    }
    out.push_str("User: ");
    out.push_str(req.user_message.trim());
    out.push_str("\n\nAssistant: ");
    out.push_str(truncated.trim());
    out
}

fn truncate_chars(s: &str, cap: usize) -> &str {
    if s.chars().count() <= cap {
        return s;
    }
    // Take `cap` chars without splitting a UTF-8 boundary.
    let end = s
        .char_indices()
        .nth(cap)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    &s[..end]
}

fn run_sync(prompt: &str, working_dir: &Path) -> Result<String, Error> {
    let mut cmd = std::process::Command::new("claude");
    cmd.arg("-p")
        .arg("--model")
        .arg(TITLE_MODEL)
        .arg(prompt)
        .current_dir(working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    SHELL_ENV.apply(&mut cmd, SHELL_ENV_TIMEOUT);

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::Spawn(format!("failed to spawn claude for title: {e}")))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::Process("no stdout from claude title subprocess".into()))?;

    let mut out = String::new();
    stdout
        .read_to_string(&mut out)
        .map_err(|e| Error::Process(format!("reading title stdout: {e}")))?;

    let status = child
        .wait()
        .map_err(|e| Error::Process(format!("waiting for title subprocess: {e}")))?;
    if !status.success() {
        return Err(Error::Process(format!(
            "claude title subprocess exited with {status}"
        )));
    }

    Ok(clean_title(&out))
}

/// Normalise the raw model output: trim whitespace, strip wrapping quotes,
/// drop trailing punctuation. Collapses to a single line in case the model
/// slipped in a newline.
fn clean_title(raw: &str) -> String {
    let single_line = raw.lines().next().unwrap_or("").trim();
    let stripped = single_line
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('`')
        .trim();
    stripped.trim_end_matches(['.', ',', ';', ':']).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_title_strips_quotes_and_punctuation() {
        assert_eq!(clean_title("\"Fixing Login Redirect.\""), "Fixing Login Redirect");
        assert_eq!(clean_title("Fixing login redirect."), "Fixing login redirect");
        assert_eq!(clean_title("'A Title'"), "A Title");
    }

    #[test]
    fn clean_title_keeps_only_first_line() {
        assert_eq!(
            clean_title("A Title\nExplanation follows"),
            "A Title",
        );
    }

    #[test]
    fn truncate_chars_respects_utf8_boundary() {
        let s = "héllo world";
        assert_eq!(truncate_chars(s, 5), "héllo");
    }

    #[test]
    fn build_prompt_omits_hint_section_when_empty() {
        let req = TitleRequest::new("hello", "hi there");
        let out = build_prompt(&req);
        assert!(!out.contains("Hint:"));
        assert!(out.contains("User: hello"));
        assert!(out.contains("Assistant: hi there"));
    }

    #[test]
    fn build_prompt_renders_hints_as_header_lines() {
        let mut req = TitleRequest::new("/ds-apply", "Let me implement step three...");
        req.context_hints
            .push("user is implementing step 03-add-login-form".into());
        req.context_hints.push("  ".into()); // empty/whitespace — should be skipped
        let out = build_prompt(&req);
        assert!(out.contains("Hint: user is implementing step 03-add-login-form"));
        // No hint for the whitespace-only entry.
        assert_eq!(out.matches("Hint:").count(), 1);
    }
}
