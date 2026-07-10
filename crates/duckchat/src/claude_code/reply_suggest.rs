//! One-shot reply suggestions via Claude Haiku (same cheap model as titles).

use std::io::Read;
use std::path::Path;
use std::process::Stdio;

use tokio::sync::oneshot;

use crate::error::Error;
use crate::reply_suggest::{
    REPLY_SUGGEST_INSTRUCTION, build_reply_suggest_prompt, parse_replies, should_skip_model,
};
use crate::request::ReplySuggestionRequest;

use super::TITLE_MODEL;
use super::spawn::claude_command;

pub async fn reply_suggestions(
    req: ReplySuggestionRequest,
    working_dir: &Path,
) -> Result<Vec<String>, Error> {
    if should_skip_model(&req) {
        return Ok(Vec::new());
    }

    let prompt = build_reply_suggest_prompt(&req);
    let working_dir = working_dir.to_path_buf();

    let (tx, rx) = oneshot::channel();
    std::thread::spawn(move || {
        let result = run_sync(&prompt, &working_dir);
        let _ = tx.send(result);
    });

    match rx.await {
        Ok(result) => result,
        Err(_) => Err(Error::Other(
            "reply suggestion thread vanished without reply".into(),
        )),
    }
}

fn run_sync(prompt: &str, working_dir: &Path) -> Result<Vec<String>, Error> {
    let mut cmd = claude_command();
    cmd.arg("-p")
        .arg("--model")
        .arg(TITLE_MODEL)
        .arg("--system-prompt")
        .arg(REPLY_SUGGEST_INSTRUCTION)
        .arg(prompt)
        .current_dir(working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::Spawn(format!("failed to spawn claude for reply suggestions: {e}")))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::Process("no stdout from claude reply-suggest subprocess".into()))?;

    let mut out = String::new();
    stdout
        .read_to_string(&mut out)
        .map_err(|e| Error::Process(format!("reading reply-suggest stdout: {e}")))?;

    let status = child
        .wait()
        .map_err(|e| Error::Process(format!("waiting for reply-suggest subprocess: {e}")))?;
    if !status.success() {
        return Err(Error::Process(format!(
            "claude reply-suggest subprocess exited with {status}"
        )));
    }

    Ok(parse_replies(&out))
}
