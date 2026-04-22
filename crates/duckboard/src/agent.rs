//! Claude Code integration for agent chat.
//!
//! Talks directly to `claude` CLI via its stream-json protocol (ndjson over
//! stdin/stdout). Each prompt turn spawns a subprocess; multi-turn is achieved
//! with `--resume <session_id>`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::Subscription;
use tokio::sync::mpsc;

// ── Public types ────────────────────────────────────────────────────────────

/// Events flowing from the Claude worker to the Iced update loop.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Worker ready — carries a handle for sending commands.
    Ready(AgentHandle),
    /// Available slash commands discovered from the project and plugins.
    CommandsAvailable(Vec<SlashCommand>),
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
    /// Usage / model info update.
    UsageUpdate {
        model: Option<String>,
        input_tokens: usize,
        output_tokens: usize,
        context_window: Option<usize>,
    },
    /// Claude Code session id for this chat — emitted after each successful
    /// turn so the app can persist it and resume across restarts.
    SessionIdUpdated { session_id: String },
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
    /// Seed the worker with a previously-persisted Claude Code session id so
    /// the next prompt resumes that conversation via `--resume <sid>`.
    SetSessionId(String),
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

/// A slash command available in the agent chat.
#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
}

/// Discover slash commands from the project and enabled plugins.
pub fn discover_commands(project_root: &Path) -> Vec<SlashCommand> {
    let mut commands = Vec::new();

    // Project-level commands: <project>/.claude/commands/*.md
    let project_cmds = project_root.join(".claude/commands");
    if project_cmds.is_dir() {
        scan_command_dir(&project_cmds, &mut commands);
    }

    // Enabled plugin commands from ~/.claude.
    if let Ok(home) = std::env::var("HOME") {
        let claude_dir = PathBuf::from(home).join(".claude");
        let settings_path = claude_dir.join("settings.json");
        if let Ok(settings_str) = std::fs::read_to_string(&settings_path)
            && let Ok(settings) = serde_json::from_str::<serde_json::Value>(&settings_str)
            && let Some(plugins) = settings["enabledPlugins"].as_object()
        {
            for (key, enabled) in plugins {
                if enabled.as_bool() != Some(true) {
                    continue;
                }
                if let Some((plugin_name, marketplace)) = key.rsplit_once('@') {
                    let plugin_dir = claude_dir
                        .join("plugins/marketplaces")
                        .join(marketplace)
                        .join("plugins")
                        .join(plugin_name);
                    let cmd_dir = plugin_dir.join("commands");
                    if cmd_dir.is_dir() {
                        scan_command_dir(&cmd_dir, &mut commands);
                    }
                    let skills_dir = plugin_dir.join("skills");
                    if skills_dir.is_dir() {
                        scan_skills_dir(&skills_dir, &mut commands);
                    }
                }
            }
        }
    }

    // Built-in Claude Code commands (not discoverable from filesystem).
    let builtins = [
        ("clear", "Clear conversation history"),
        ("compact", "Summarize and compact conversation"),
        ("cost", "Show token usage and cost"),
        ("help", "Show available commands"),
        ("model", "Switch the model"),
    ];
    for (name, desc) in builtins {
        commands.push(SlashCommand {
            name: name.into(),
            description: desc.into(),
        });
    }

    commands.sort_by(|a, b| a.name.cmp(&b.name));
    commands.dedup_by(|a, b| a.name == b.name);
    commands
}

fn scan_command_dir(dir: &Path, commands: &mut Vec<SlashCommand>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let description = parse_frontmatter_description(&path).unwrap_or_default();
        commands.push(SlashCommand { name, description });
    }
}

fn scan_skills_dir(dir: &Path, commands: &mut Vec<SlashCommand>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_file = path.join("SKILL.md");
        if !skill_file.exists() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let description = parse_frontmatter_description(&skill_file).unwrap_or_default();
        commands.push(SlashCommand { name, description });
    }
}

fn parse_frontmatter_description(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let body = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))?;
    let end = body.find("\n---")?;
    let frontmatter = &body[..end];
    for line in frontmatter.lines() {
        if let Some(desc) = line.strip_prefix("description:") {
            let desc = desc.trim().trim_matches('"');
            if !desc.is_empty() {
                return Some(desc.to_string());
            }
        }
    }
    None
}

// ── Protocol types ─────────────────────────────────────────────────────────

/// Top-level protocol message from `claude -p --output-format stream-json`.
#[derive(Debug, serde::Deserialize)]
struct ProtocolMsg {
    #[serde(rename = "type")]
    type_: String,
    // stream_event
    event: Option<StreamEvent>,
    // assistant / tool_result / user
    message: Option<MessageBody>,
    // system
    model: Option<String>,
    // result
    session_id: Option<String>,
    #[serde(rename = "modelUsage")]
    model_usage: Option<serde_json::Value>,
    is_error: Option<bool>,
    result: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
struct StreamEvent {
    #[serde(rename = "type")]
    type_: String,
    delta: Option<DeltaBlock>,
}

#[derive(Debug, serde::Deserialize)]
struct DeltaBlock {
    text: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct MessageBody {
    content: Option<Vec<ContentBlock>>,
    usage: Option<UsageBlock>,
}

#[derive(Debug, serde::Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    type_: String,
    // tool_use
    id: Option<String>,
    name: Option<String>,
    input: Option<serde_json::Value>,
    // tool_result
    tool_use_id: Option<String>,
    content: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct UsageBlock {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
}

/// Parse a single protocol line into zero or more AgentEvents.
fn parse_protocol_line(msg: &ProtocolMsg) -> Vec<AgentEvent> {
    let mut events = Vec::new();

    match msg.type_.as_str() {
        "stream_event" => {
            if let Some(event) = &msg.event
                && event.type_ == "content_block_delta"
                && let Some(delta) = &event.delta
                && let Some(text) = &delta.text
            {
                events.push(AgentEvent::ContentDelta { text: text.clone() });
            }
        }
        "assistant" => {
            if let Some(body) = &msg.message {
                if let Some(usage) = &body.usage {
                    // Assistant messages report per-request usage. Summing all
                    // three input fields gives the prompt size at this turn —
                    // i.e. current context-window fill. (Do NOT use the `result`
                    // message for this: its usage is cumulative across every
                    // internal model call, which inflates `cache_read` N-fold
                    // when the agent loops through tool use.)
                    let input_t = (usage.input_tokens
                        + usage.cache_read_input_tokens
                        + usage.cache_creation_input_tokens)
                        as usize;
                    let output_t = usage.output_tokens as usize;
                    if input_t > 0 || output_t > 0 {
                        events.push(AgentEvent::UsageUpdate {
                            model: None,
                            input_tokens: input_t,
                            output_tokens: output_t,
                            context_window: None,
                        });
                    }
                }
                if let Some(content) = &body.content {
                    for block in content {
                        if block.type_ == "tool_use" {
                            events.push(AgentEvent::ToolUse {
                                id: block.id.clone().unwrap_or_default(),
                                name: block.name.clone().unwrap_or_default(),
                                input: block
                                    .input
                                    .as_ref()
                                    .map_or(String::new(), |v| v.to_string()),
                            });
                        }
                    }
                }
            }
        }
        "tool_result" | "user" => {
            if let Some(body) = &msg.message
                && let Some(content) = &body.content
            {
                for block in content {
                    if block.type_ == "tool_result" {
                        let output = block.content.as_ref().map_or(String::new(), |v| {
                            v.as_str().map_or_else(|| v.to_string(), |s| s.to_string())
                        });
                        events.push(AgentEvent::ToolResult {
                            id: block.tool_use_id.clone().unwrap_or_default(),
                            name: String::new(),
                            output,
                        });
                    }
                }
            }
        }
        "system" => {
            if let Some(model) = &msg.model {
                events.push(AgentEvent::UsageUpdate {
                    model: Some(model.clone()),
                    input_tokens: 0,
                    output_tokens: 0,
                    context_window: None,
                });
            }
        }
        "result" => {
            // Only propagate the context-window capacity here. `msg.usage` on
            // result messages is cumulative across every internal model call
            // in the turn (with prompt caching that multiplies `cache_read`
            // several-fold), so it's unusable as a current-prompt-size signal.
            let context_window = msg
                .model_usage
                .as_ref()
                .and_then(|mu| mu.as_object())
                .and_then(|mu| mu.values().next())
                .and_then(|v| v["contextWindow"].as_u64())
                .map(|v| v as usize);

            if let Some(cw) = context_window {
                events.push(AgentEvent::UsageUpdate {
                    model: None,
                    input_tokens: 0,
                    output_tokens: 0,
                    context_window: Some(cw),
                });
            }
        }
        _ => {}
    }

    events
}

/// Clonable handle for sending commands to the worker.
#[derive(Clone)]
pub struct AgentHandle {
    cancel_flag: Arc<AtomicBool>,
    tx: mpsc::UnboundedSender<AgentCommand>,
}

impl AgentHandle {
    pub fn send_prompt(&self, text: String, context: Option<AgentContext>) {
        let _ = self.tx.send(AgentCommand::SendPrompt { text, context });
    }

    pub fn set_session_id(&self, session_id: String) {
        let _ = self.tx.send(AgentCommand::SetSessionId(session_id));
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
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
/// The `key` parameter makes each subscription unique so multiple agents can coexist.
/// Returns `(key, event)` tuples so the caller can route events without capturing in `.map()`.
pub fn agent_subscription(
    key: String,
    project_root: PathBuf,
) -> Subscription<(String, AgentEvent)> {
    Subscription::run_with((key, project_root.clone()), |(key, root)| {
        use iced::futures::StreamExt;
        let key = key.clone();
        agent_worker(root.clone()).map(move |e| (key.clone(), e))
    })
}

fn agent_worker(project_root: PathBuf) -> impl iced::futures::Stream<Item = AgentEvent> {
    iced::stream::channel(
        256,
        |mut sender: iced::futures::channel::mpsc::Sender<AgentEvent>| async move {
            use iced::futures::SinkExt;

            let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<AgentCommand>();

            // Shared cancel flag — set by cancel(), checked by read loop.
            let cancel_flag = Arc::new(AtomicBool::new(false));

            // Send handle to Iced immediately.
            let handle = AgentHandle {
                cancel_flag: cancel_flag.clone(),
                tx: cmd_tx,
            };
            if sender.send(AgentEvent::Ready(handle)).await.is_err() {
                return;
            }

            // Discover available slash commands and send them.
            let commands = discover_commands(&project_root);
            if !commands.is_empty() {
                let _ = sender.send(AgentEvent::CommandsAvailable(commands)).await;
            }

            // Session ID persists across turns for multi-turn conversations.
            let mut session_id: Option<String> = None;

            // Command loop — each prompt spawns a new `claude -p` subprocess.
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    AgentCommand::SendPrompt { text, context } => {
                        cancel_flag.store(false, Ordering::SeqCst);
                        let prompt = format_prompt(&text, context.as_ref());
                        match run_prompt_turn(
                            &project_root,
                            &prompt,
                            session_id.as_deref(),
                            &mut sender,
                            &cancel_flag,
                        )
                        .await
                        {
                            Ok(new_session_id) => {
                                let changed =
                                    session_id.as_deref() != Some(new_session_id.as_str());
                                session_id = Some(new_session_id.clone());
                                if changed
                                    && sender
                                        .send(AgentEvent::SessionIdUpdated {
                                            session_id: new_session_id,
                                        })
                                        .await
                                        .is_err()
                                {
                                    break;
                                }
                                if sender.send(AgentEvent::TurnComplete).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                let msg = format!("{e}");
                                if msg != "cancelled" {
                                    let _ = sender.send(AgentEvent::Error(msg)).await;
                                } else {
                                    let _ = sender.send(AgentEvent::TurnComplete).await;
                                }
                            }
                        }
                    }
                    AgentCommand::SetSessionId(sid) => {
                        session_id = Some(sid);
                    }
                    AgentCommand::Shutdown => {
                        tracing::info!("agent chat shutdown");
                        cancel_flag.store(true, Ordering::SeqCst);
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
    cancel_flag: &Arc<AtomicBool>,
) -> anyhow::Result<String> {
    use iced::futures::SinkExt;
    use std::io::Write;

    let mut cmd = std::process::Command::new("claude");
    cmd.arg("-p")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--include-partial-messages")
        .arg("--permission-mode")
        .arg("bypassPermissions")
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
        // Check cancel flag between lines.
        if cancel_flag.load(Ordering::SeqCst) {
            tracing::info!("cancelling agent turn, killing child");
            let _ = child.kill();
            // Drain remaining lines.
            while line_rx.recv().await.is_some() {}
            return Err(anyhow::anyhow!("cancelled"));
        }

        let Some(line) = data else { break }; // EOF
        if line.is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<ProtocolMsg>(&line) else {
            continue;
        };

        // Handle result-level fields (session_id, is_error) that don't map
        // to AgentEvent variants cleanly.
        if msg.type_ == "result" {
            tracing::debug!(result_line = %line, "agent result message");
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
                return Err(anyhow::anyhow!("{error_msg}"));
            }
        }

        for event in parse_protocol_line(&msg) {
            let _ = sender.send(event).await;
        }
    }

    // Wait for process to finish.
    std::thread::spawn(move || {
        child.wait().ok();
    });

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
