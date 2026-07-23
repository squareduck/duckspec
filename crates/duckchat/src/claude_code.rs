//! Claude Code harness.
//!
//! Thin provider over the shared [`crate::acp`] client. Builds a
//! [`duckchat-claude-acp`](crate::claude_code::claude_acp_launch) [`AgentLaunch`]
//! and opens shared main/oneshot runtimes. Model aliases, command discovery,
//! and title/reply prompt helpers stay harness-local. Stream-json to the
//! official `claude` CLI lives only inside the owned agent process.

mod agent_bin;
mod discover;

pub use agent_bin::{CLAUDE_ACP_BIN, CLAUDE_ACP_ENV, claude_acp_launch, resolve_claude_acp_binary};

/// Project/plugin slash-command discovery. Re-exported for the grok harness,
/// which loads the same `.claude` skills and commands.
pub(crate) use discover::discover_commands;

use std::path::Path;
use std::sync::OnceLock;

use async_trait::async_trait;

use crate::acp::{AcpMainRuntime, AcpModel, AcpOneshotRuntime, AcpTurn, AgentLaunch};
use crate::error::Error;
use crate::provider::{Capabilities, ModelInfo, Provider, SlashCommand};
use crate::reply_suggest::{
    REPLY_SUGGEST_INSTRUCTION, build_reply_suggest_prompt, parse_replies, should_skip_model,
};
use crate::request::{ReplySuggestionRequest, TitleRequest};
use crate::runtime::{MainRuntime, OneshotKind, OneshotRuntime};
use crate::title::{build_title_prompt, clean_title};

/// Stable harness id shared by every model this provider owns.
const HARNESS: &str = "claude-code";

/// Preferred model for one-shot title / reply-suggest calls. Matches the
/// curated `haiku` alias the agent advertises on initialize.
const TITLE_MODEL: &str = "haiku";

/// [`Provider`] over the owned Claude ACP agent (`duckchat-claude-acp`). Models
/// are discovered once from the agent's ACP `initialize` handshake and cached
/// for the provider instance lifetime.
#[derive(Clone)]
pub struct ClaudeCodeProvider {
    launch: AgentLaunch,
    models: OnceLock<Vec<ModelInfo>>,
}

impl ClaudeCodeProvider {
    pub fn new() -> Self {
        Self {
            launch: claude_acp_launch(),
            models: OnceLock::new(),
        }
    }

    /// Construct with a custom launch. Test-only seam for driving the provider
    /// against a scripted or missing agent binary.
    #[cfg(test)]
    fn with_launch(launch: AgentLaunch) -> Self {
        Self {
            launch,
            models: OnceLock::new(),
        }
    }

    /// Discover models from a fresh `initialize` handshake. Synchronous to fit
    /// [`Provider::list_models`]: runs the async handshake on a dedicated
    /// thread with its own runtime. Failure degrades to an empty list.
    fn discover_models(&self) -> Vec<ModelInfo> {
        let launch = self.launch.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(_) => return Vec::new(),
            };
            rt.block_on(async move {
                let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                let mut turn = match AcpTurn::spawn_with(&launch, &cwd).await {
                    Ok(turn) => turn,
                    Err(_) => return Vec::new(),
                };
                let models = match turn.initialize().await {
                    Ok(init) => init.models.into_iter().map(to_model_info).collect(),
                    Err(_) => Vec::new(),
                };
                turn.cancel().await;
                models
            })
        })
        .join()
        .unwrap_or_default()
    }
}

impl Default for ClaudeCodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Map a handshake-advertised model onto neutral [`ModelInfo`] for Claude.
fn to_model_info(m: AcpModel) -> ModelInfo {
    ModelInfo {
        harness: HARNESS.to_string(),
        id: m.id.clone(),
        display: humanize_display(&m.id, &m.name),
        context_window: m.context_window,
    }
}

/// Prefer a human advertised name; otherwise map known aliases or light-prettify.
fn humanize_display(id: &str, advertised: &str) -> String {
    let advertised = advertised.trim();
    if !advertised.is_empty() && advertised != id {
        return advertised.to_string();
    }
    match id {
        "fable" => "Fable".into(),
        "opus" => "Opus".into(),
        "sonnet" => "Sonnet".into(),
        "haiku" => "Haiku".into(),
        other => {
            // claude-opus-4-8 → Claude Opus 4.8-ish light prettify
            let stripped = other.strip_prefix("claude-").unwrap_or(other);
            stripped
                .split('-')
                .filter(|s| !s.is_empty())
                .map(|part| {
                    if part.chars().all(|c| c.is_ascii_digit()) {
                        part.to_string()
                    } else if part.len() == 1 {
                        part.to_uppercase()
                    } else {
                        let mut c = part.chars();
                        match c.next() {
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                            None => String::new(),
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
}

#[async_trait]
impl Provider for ClaudeCodeProvider {
    fn id(&self) -> &str {
        HARNESS
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            streaming: true,
            tool_use: true,
            resume: true,
            reasoning: false,
            slash_commands: true,
        }
    }

    fn list_models(&self) -> Vec<ModelInfo> {
        self.models.get_or_init(|| self.discover_models()).clone()
    }

    fn list_commands(&self, project_root: &Path) -> Vec<SlashCommand> {
        discover::discover_commands(project_root)
    }

    fn open_main_runtime(&self, working_dir: &Path) -> Box<dyn MainRuntime> {
        Box::new(AcpMainRuntime::new(self.launch.clone(), working_dir))
    }

    fn open_oneshot_runtime(
        &self,
        working_dir: &Path,
        preferred_model: Option<String>,
    ) -> Box<dyn OneshotRuntime> {
        // Host passes a catalog-resolved id when available. Bare aliases (e.g.
        // TITLE_MODEL) still work via `pick_oneshot_model` substring match
        // against live full API ids — do not inject a bare alias when None.
        Box::new(AcpOneshotRuntime::with_preferred_model(
            self.launch.clone(),
            working_dir,
            preferred_model,
        ))
    }

    async fn title_summary(&self, req: TitleRequest, working_dir: &Path) -> Result<String, Error> {
        // Prefer cheap/fast needle; pick_oneshot_model matches full advertise ids.
        let mut rt = self.open_oneshot_runtime(working_dir, Some(TITLE_MODEL.to_string()));
        let raw = rt
            .prompt(OneshotKind::Title, build_title_prompt(&req))
            .await?;
        Ok(clean_title(&raw))
    }

    async fn reply_suggestions(
        &self,
        req: ReplySuggestionRequest,
        working_dir: &Path,
    ) -> Result<Vec<String>, Error> {
        if should_skip_model(&req) {
            return Ok(Vec::new());
        }

        let mut rt = self.open_oneshot_runtime(working_dir, Some(TITLE_MODEL.to_string()));
        let body = build_reply_suggest_prompt(&req);
        let text = format!("{REPLY_SUGGEST_INSTRUCTION}\n\n{body}");
        let raw = rt.prompt(OneshotKind::ReplySuggest, text).await?;
        Ok(parse_replies(&raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::AgentLaunch;
    use crate::cancel::CancelToken;
    use crate::event::AgentEvent;
    use crate::request::TurnRequest;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use tempfile::TempDir;
    use tokio::process::Command;
    use tokio::sync::mpsc;

    /// Serialize env-touching / PATH-sensitive tests.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn write_executable(path: &std::path::Path, body: &str) {
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        f.flush().unwrap();
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).unwrap();
    }

    /// Minimal ACP agent peer that answers initialize / session/new / prompt.
    fn install_fake_acp_agent(dir: &TempDir) -> PathBuf {
        install_fake_acp_agent_with_models(dir, r#"[{"modelId": "haiku", "name": "Haiku 4.5"}]"#)
    }

    /// Fake ACP agent whose initialize advertises the given `availableModels` JSON array.
    fn install_fake_acp_agent_with_models(dir: &TempDir, models_json: &str) -> PathBuf {
        let path = dir.path().join("fake-claude-acp");
        let script = format!(
            r#"#!/usr/bin/env python3
import json, sys

MODELS = {models_json}

def send(msg):
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()

def respond(req_id, result):
    send({{"jsonrpc": "2.0", "id": req_id, "result": result}})

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    msg = json.loads(line)
    method = msg.get("method")
    req_id = msg.get("id")
    if method == "initialize":
        respond(req_id, {{
            "protocolVersion": 1,
            "agentCapabilities": {{"loadSession": True}},
            "_meta": {{"modelState": {{"availableModels": MODELS}}}},
        }})
    elif method == "session/new":
        respond(req_id, {{"sessionId": "claude-native-sess-1"}})
    elif method == "session/load":
        respond(req_id, {{"sessionId": msg.get("params", {{}}).get("sessionId", "loaded")}})
    elif method == "session/prompt":
        send({{
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {{
                "sessionId": "claude-native-sess-1",
                "update": {{
                    "sessionUpdate": "agent_message_chunk",
                    "content": {{"type": "text", "text": "from-owned-agent"}},
                }},
            }},
        }})
        respond(req_id, {{"stopReason": "end_turn"}})
    elif method == "session/cancel":
        pass
    elif req_id is not None:
        respond(req_id, {{}})
"#
        );
        write_executable(&path, &script);
        path
    }

    /// @spec harness/claude Owned ACP agent over official Claude CLI: A Claude turn is driven through the owned ACP agent process
    #[tokio::test]
    async fn claude_turn_driven_through_owned_acp_agent() {
        let tmp = TempDir::new().unwrap();
        let provider = {
            let _guard = ENV_LOCK.lock().unwrap();
            let agent = install_fake_acp_agent(&tmp);

            // Launch is the owned agent binary — not `claude -p` stream-json.
            let launch = AgentLaunch::new({
                let agent = agent.clone();
                move || Command::new(&agent)
            });
            let provider = ClaudeCodeProvider::with_launch(launch);

            let cmd = claude_acp_launch().command();
            let program = cmd.as_std().get_program().to_string_lossy().into_owned();
            let args: Vec<String> = cmd
                .as_std()
                .get_args()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
            // Default launch targets the owned agent name (resolved or PATH fallthrough).
            assert!(
                program.contains(CLAUDE_ACP_BIN) || args.iter().any(|a| a.contains(CLAUDE_ACP_BIN)),
                "default Claude launch must target {CLAUDE_ACP_BIN}, got program={program} args={args:?}"
            );
            assert!(
                !args.iter().any(|a| a == "-p" || a == "stream-json"),
                "host must not drive claude via in-host stream-json flags: {args:?}"
            );
            provider
        };

        // A turn completes through the shared ACP main runtime against the agent.
        let mut rt = provider.open_main_runtime(tmp.path());
        let (tx, mut rx) = mpsc::channel(16);
        let req = TurnRequest::new("hello", tmp.path().to_path_buf());
        let outcome = rt
            .run_turn(
                req,
                tx,
                CancelToken::new(),
                crate::event::PendingUserChoices::shared(),
            )
            .await
            .expect("turn through owned ACP agent");
        assert_eq!(outcome.session_id, "claude-native-sess-1");

        let mut saw_content = false;
        while let Ok(ev) = rx.try_recv() {
            if let AgentEvent::ContentDelta { text } = ev
                && text.contains("from-owned-agent")
            {
                saw_content = true;
            }
        }
        assert!(
            saw_content,
            "host ACP client must receive profile content from the owned agent"
        );
    }

    /// Sanity: title/reply stay on the shared oneshot path (N=1 preferred haiku).
    #[test]
    fn oneshot_runtime_uses_shared_acp_path() {
        let provider = ClaudeCodeProvider::new();
        // Construction must not panic; preferred model is the curated haiku alias.
        let _rt = provider.open_oneshot_runtime(std::path::Path::new("/tmp"), None);
        assert_eq!(TITLE_MODEL, "haiku");
        assert_eq!(provider.id(), "claude-code");
        assert!(!provider.capabilities().reasoning);
    }

    /// @spec harness/claude Model discovery: Listed models come from the agent advertise set
    #[test]
    fn listed_models_come_from_the_agent_advertise_set() {
        let tmp = TempDir::new().unwrap();
        let agent = install_fake_acp_agent_with_models(
            &tmp,
            r#"[
                {"modelId": "opus", "name": "Opus 4.8"},
                {"modelId": "sonnet", "name": "Sonnet 4.6"}
            ]"#,
        );
        let provider = ClaudeCodeProvider::with_launch(AgentLaunch::new({
            let agent = agent.clone();
            move || Command::new(&agent)
        }));

        // GIVEN the owned Claude agent advertising a set of available models
        // WHEN the harness lists models
        let listed = provider.list_models();

        // THEN the listed models are exactly that advertised set
        // AND each listed model is tagged with the Claude harness
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, "opus");
        assert_eq!(listed[1].id, "sonnet");
        assert!(listed.iter().all(|m| m.harness == "claude-code"));
    }

    /// @spec harness/claude Model discovery: Each listed model carries a display name
    #[test]
    fn each_listed_model_carries_a_display_name() {
        // GIVEN the owned Claude agent advertising models with display names
        let handshake = vec![
            AcpModel {
                id: "opus".into(),
                name: "Opus 4.8".into(),
                context_window: None,
            },
            AcpModel {
                id: "haiku".into(),
                name: "haiku".into(), // ugly: same as id → humanize alias
                context_window: None,
            },
        ];

        // WHEN the harness lists models
        let listed: Vec<ModelInfo> = handshake.into_iter().map(to_model_info).collect();

        // THEN each listed model carries a non-empty display name
        assert!(listed.iter().all(|m| !m.display.is_empty()));
        assert_eq!(listed[0].display, "Opus 4.8");
        assert_eq!(listed[1].display, "Haiku");
    }

    /// @spec harness/claude Model discovery: A model with a known context window carries that window
    #[test]
    fn model_with_known_context_window_carries_that_window() {
        // GIVEN the owned Claude agent advertising a model with a known context window
        let handshake = vec![AcpModel {
            id: "claude-opus-4-8".into(),
            name: "Claude Opus 4.8".into(),
            context_window: Some(1_000_000),
        }];

        // WHEN the harness lists models
        let listed: Vec<ModelInfo> = handshake.into_iter().map(to_model_info).collect();

        // THEN that listed model carries the same context window
        assert_eq!(listed[0].context_window, Some(1_000_000));
    }

    /// @spec harness/claude Model discovery: Discovery failure yields an empty host list without panic
    #[test]
    fn discovery_failure_yields_empty_host_list_without_panic() {
        // GIVEN an environment where Claude model discovery cannot obtain an advertised set
        let provider = ClaudeCodeProvider::with_launch(AgentLaunch::new(|| {
            Command::new("/nonexistent/duckchat-claude-acp-does-not-exist")
        }));

        // WHEN the harness lists models
        let listed = provider.list_models();

        // THEN the model list is empty
        // AND the listing completes without panicking
        assert!(listed.is_empty());
    }
}
