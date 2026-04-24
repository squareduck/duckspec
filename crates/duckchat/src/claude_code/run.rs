//! Spawns `claude -p` and drives a single turn.

use std::time::Duration;

use tokio::sync::mpsc;

use crate::cancel::CancelToken;
use crate::error::Error;
use crate::event::AgentEvent;
use crate::request::{ToolPolicy, TurnOutcome, TurnRequest};
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

    let prompt = assemble_prompt(&req);

    let mut cmd = std::process::Command::new("claude");
    cmd.arg("-p")
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

    SHELL_ENV.apply(&mut cmd, SHELL_ENV_TIMEOUT);

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::Spawn(format!("failed to spawn claude: {e}")))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes()).ok();
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

/// Prepend `system_additions` (separated by blank lines) before the user's
/// prompt text. Returns just the prompt when there are no additions.
fn assemble_prompt(req: &TurnRequest) -> String {
    if req.system_additions.is_empty() {
        return req.prompt.clone();
    }
    let mut out = String::new();
    for addition in &req.system_additions {
        if addition.is_empty() {
            continue;
        }
        out.push_str(addition);
        out.push_str("\n\n");
    }
    out.push_str(&req.prompt);
    out
}
