//! Claude Code integration for agent chat.
//!
//! Talks directly to `claude` CLI via its stream-json protocol (ndjson over
//! stdin/stdout). Each prompt turn spawns a subprocess; multi-turn is achieved
//! with `--resume <session_id>`.

use std::path::PathBuf;

use iced::Subscription;
use tokio::sync::mpsc;

// ── Public types ────────────────────────────────────────────────────────────

/// Events flowing from the Claude worker to the Iced update loop.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Worker ready — carries a handle for sending commands.
    Ready(AgentHandle),
    /// Streaming text chunk from the agent.
    ContentDelta { text: String },
    /// Agent started a tool call.
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    /// Agent tool call completed with output.
    ToolResult {
        id: String,
        name: String,
        output: String,
    },
    /// Agent finished its turn.
    TurnComplete,
    /// An error occurred.
    Error(String),
    /// The worker exited.
    ProcessExited,
}

/// Commands sent from the Iced update loop to the worker.
#[derive(Debug)]
pub enum AgentCommand {
    SendPrompt {
        text: String,
        context: Option<AgentContext>,
    },
    Cancel,
    Shutdown,
}

/// Context injected into the agent session.
#[derive(Debug, Clone)]
pub struct AgentContext {
    pub project_root: PathBuf,
    pub change_dir: PathBuf,
    pub changed_files: Vec<PathBuf>,
    pub spec_content: Option<String>,
    pub step_content: Option<String>,
    pub git_diff: Option<String>,
}

/// Clonable handle for sending commands to the worker.
#[derive(Clone)]
pub struct AgentHandle {
    tx: mpsc::UnboundedSender<AgentCommand>,
}

impl AgentHandle {
    pub fn send_prompt(&self, text: String, context: Option<AgentContext>) {
        let _ = self.tx.send(AgentCommand::SendPrompt { text, context });
    }

    pub fn cancel(&self) {
        let _ = self.tx.send(AgentCommand::Cancel);
    }

    #[allow(dead_code)]
    pub fn shutdown(&self) {
        let _ = self.tx.send(AgentCommand::Shutdown);
    }
}

impl std::fmt::Debug for AgentHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AgentHandle")
    }
}

// ── Subscription ────────────────────────────────────────────────────────────

/// Create a subscription that manages a Claude Code agent chat session.
pub fn agent_subscription(project_root: PathBuf) -> Subscription<AgentEvent> {
    Subscription::run_with(project_root, |root| agent_worker(root.clone()))
}

fn agent_worker(project_root: PathBuf) -> impl iced::futures::Stream<Item = AgentEvent> {
    iced::stream::channel(
        256,
        |mut sender: iced::futures::channel::mpsc::Sender<AgentEvent>| async move {
            use iced::futures::SinkExt;

            let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<AgentCommand>();

            // Send handle to Iced immediately.
            let handle = AgentHandle { tx: cmd_tx };
            if sender.send(AgentEvent::Ready(handle)).await.is_err() {
                return;
            }

            // Session ID persists across turns for multi-turn conversations.
            let mut session_id: Option<String> = None;

            // Command loop — each prompt spawns a new `claude -p` subprocess.
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    AgentCommand::SendPrompt { text, context } => {
                        let prompt = format_prompt(&text, context.as_ref());
                        match run_prompt_turn(
                            &project_root,
                            &prompt,
                            session_id.as_deref(),
                            &mut sender,
                        )
                        .await
                        {
                            Ok(new_session_id) => {
                                session_id = Some(new_session_id);
                                if sender.send(AgentEvent::TurnComplete).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                let _ = sender.send(AgentEvent::Error(format!("{e}"))).await;
                            }
                        }
                    }
                    AgentCommand::Cancel => {
                        // Can't cancel a -p subprocess cleanly; user can send next message.
                        tracing::info!("cancel requested (ignored for -p mode)");
                    }
                    AgentCommand::Shutdown => {
                        tracing::info!("agent chat shutdown");
                        break;
                    }
                }
            }

            let _ = sender.send(AgentEvent::ProcessExited).await;
        },
    )
}

/// Run a single prompt turn by spawning `claude -p` and streaming its output.
/// Returns the session ID from the result message (for `--resume` on next turn).
///
/// Uses `std::process` with a background reader thread (not tokio) because
/// Iced's async runtime is not tokio, so `tokio::process` panics.
async fn run_prompt_turn(
    project_root: &PathBuf,
    prompt: &str,
    resume_session: Option<&str>,
    sender: &mut iced::futures::channel::mpsc::Sender<AgentEvent>,
) -> anyhow::Result<String> {
    use iced::futures::SinkExt;
    use std::io::Write;

    let mut cmd = std::process::Command::new("claude");
    cmd.arg("-p")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--include-partial-messages")
        .current_dir(project_root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());

    if let Some(sid) = resume_session {
        cmd.arg("--resume").arg(sid);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn claude: {e}"))?;

    // Write prompt to stdin and close it.
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes()).ok();
        // stdin drops here, closing the pipe.
    }

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("no stdout"))?;

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
        let Some(line) = data else { break }; // EOF
        if line.is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };

        let msg_type = msg["type"].as_str().unwrap_or("");

        match msg_type {
            // Streaming text deltas from --include-partial-messages.
            "stream_event" => {
                if let Some(event) = msg.get("event") {
                    let event_type = event["type"].as_str().unwrap_or("");
                    if event_type == "content_block_delta" {
                        if let Some(text) = event["delta"]["text"].as_str() {
                            let _ = sender
                                .send(AgentEvent::ContentDelta {
                                    text: text.to_string(),
                                })
                                .await;
                        }
                    }
                }
            }
            // Complete assistant message (contains tool_use blocks too).
            "assistant" => {
                if let Some(content) = msg["message"]["content"].as_array() {
                    for block in content {
                        match block["type"].as_str().unwrap_or("") {
                            "tool_use" => {
                                let _ = sender
                                    .send(AgentEvent::ToolUse {
                                        id: block["id"]
                                            .as_str()
                                            .unwrap_or("")
                                            .to_string(),
                                        name: block["name"]
                                            .as_str()
                                            .unwrap_or("")
                                            .to_string(),
                                        input: block["input"].to_string(),
                                    })
                                    .await;
                            }
                            _ => {}
                        }
                    }
                }
            }
            // Tool result from Claude executing a tool.
            "tool_result" | "user" => {
                if let Some(content) = msg["message"]["content"].as_array() {
                    for block in content {
                        if block["type"].as_str() == Some("tool_result") {
                            let _ = sender
                                .send(AgentEvent::ToolResult {
                                    id: block["tool_use_id"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string(),
                                    name: String::new(),
                                    output: block["content"]
                                        .as_str()
                                        .unwrap_or(&block["content"].to_string())
                                        .to_string(),
                                })
                                .await;
                        }
                    }
                }
            }
            // Final result — contains session_id for resume.
            "result" => {
                if let Some(sid) = msg["session_id"].as_str() {
                    result_session_id = sid.to_string();
                }
                if msg["is_error"].as_bool() == Some(true) {
                    let error_msg = msg["result"]
                        .as_str()
                        .unwrap_or("unknown error")
                        .to_string();
                    return Err(anyhow::anyhow!("{error_msg}"));
                }
            }
            _ => {}
        }
    }

    // Wait for process to finish.
    std::thread::spawn(move || { child.wait().ok(); });

    if result_session_id.is_empty() {
        Err(anyhow::anyhow!("no session_id in result"))
    } else {
        Ok(result_session_id)
    }
}

/// Format a user prompt, optionally prepending change context.
fn format_prompt(user_text: &str, context: Option<&AgentContext>) -> String {
    let Some(ctx) = context else {
        return user_text.to_string();
    };

    let mut parts = vec![];

    parts.push(format!(
        "Project root: {}\nChange directory: {}",
        ctx.project_root.display(),
        ctx.change_dir.display(),
    ));

    if !ctx.changed_files.is_empty() {
        let files: Vec<_> = ctx
            .changed_files
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        parts.push(format!("Changed files:\n{}", files.join("\n")));
    }

    if let Some(spec) = &ctx.spec_content {
        parts.push(format!("Change spec:\n{spec}"));
    }

    if let Some(step) = &ctx.step_content {
        parts.push(format!("Current step:\n{step}"));
    }

    if let Some(diff) = &ctx.git_diff {
        parts.push(format!("Git diff:\n```\n{diff}\n```"));
    }

    parts.push(format!("User request:\n{user_text}"));
    parts.join("\n\n")
}
