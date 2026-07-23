//! `duckchat-claude-acp` — ACP server on stdio wrapping the official `claude` CLI.
//!
//! Speaks the shared duckchat client dialect (`initialize`, `session/new`,
//! `session/load`, `session/prompt`, cancel) and holds a duplex-hot Claude
//! process across main turns when possible.

mod agent;
mod claude;
mod models;

use std::io::Write;
use std::process::ExitCode;

use agent::{Agent, AgentError};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader, stdin};
use tracing::warn;

#[tokio::main]
async fn main() -> ExitCode {
    // Log to stderr so stdout stays JSON-RPC clean for the ACP parent.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::WARN)
        .init();

    if let Err(e) = run().await {
        eprintln!("duckchat-claude-acp: {e:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

async fn run() -> anyhow::Result<()> {
    let mut reader = BufReader::new(stdin());
    let mut agent = Agent::new();
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            // Parent closed stdin — tear down Claude heat and exit.
            agent.cancel(None).await;
            break;
        }

        let msg: Value = match serde_json::from_str(line.trim_end()) {
            Ok(v) => v,
            Err(e) => {
                warn!("malformed json-rpc line: {e}");
                continue;
            }
        };

        // Notifications (method, no id) — ignore in this profile.
        let Some(id) = msg.get("id").cloned() else {
            continue;
        };

        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        match method {
            "initialize" => {
                write_result(id, agent.initialize().await)?;
            }
            "session/new" => match agent.session_new(&params).await {
                Ok(result) => write_result(id, result)?,
                Err(err) => write_error(id, &err)?,
            },
            "session/load" => match agent.session_load(&params).await {
                Ok(result) => write_result(id, result)?,
                Err(err) => write_error(id, &err)?,
            },
            "session/prompt" => {
                // Stream profile updates live while Claude runs; mid-prompt
                // AskUserQuestion issues session/request_permission and reads
                // the parent answer from the same stdin. All ACP lines share
                // one stdout path (`write_acp_value`).
                let mut emit_err: Option<anyhow::Error> = None;
                let mut on_update = |update: Value| {
                    if emit_err.is_some() {
                        return;
                    }
                    if let Err(e) = write_notification("session/update", update) {
                        emit_err = Some(e);
                    }
                };
                let mut write_parent = |msg: Value| -> Result<(), AgentError> {
                    write_acp_value(&msg)
                        .map_err(|e| AgentError::Process(format!("write parent acp: {e}")))
                };
                match agent
                    .run_prompt(&params, &mut on_update, &mut reader, &mut write_parent)
                    .await
                {
                    Ok(result) => {
                        if let Some(e) = emit_err {
                            return Err(e);
                        }
                        write_result(id, result)?;
                    }
                    Err(err) => {
                        if let Some(e) = emit_err {
                            return Err(e);
                        }
                        write_error(id, &err)?;
                    }
                }
            }
            "session/cancel" => {
                agent
                    .cancel(params.get("sessionId").and_then(Value::as_str))
                    .await;
                write_result(id, json!({}))?;
            }
            other => {
                write_error(id, &AgentError::MethodNotFound(other.into()))?;
            }
        }
    }

    Ok(())
}

fn write_result(id: Value, result: Value) -> anyhow::Result<()> {
    write_acp(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    }))
}

fn write_error(id: Value, err: &AgentError) -> anyhow::Result<()> {
    write_acp(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": err.to_rpc_value(),
    }))
}

fn write_notification(method: &str, params: Value) -> anyhow::Result<()> {
    write_acp(&json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    }))
}

/// Single stdout write path for every ACP JSON-RPC line (results, errors,
/// mid-turn `session/update` notifications, and agent→parent requests).
/// Locked flush so live lines leave the process before the next write.
fn write_acp(msg: &Value) -> anyhow::Result<()> {
    write_acp_value(msg)
}

fn write_acp_value(msg: &Value) -> anyhow::Result<()> {
    let mut line = serde_json::to_string(msg)?;
    line.push('\n');
    let mut out = std::io::stdout().lock();
    out.write_all(line.as_bytes())?;
    out.flush()?;
    Ok(())
}
