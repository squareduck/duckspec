//! OpenAI Codex harness.
//!
//! Thin provider over the shared [`crate::acp`] client. Builds a
//! [`duckchat-codex-acp`](crate::openai_codex::codex_acp_launch) [`AgentLaunch`]
//! and opens shared main/oneshot runtimes. Skill discovery and oneshot
//! preference stay harness-local. App Server lives only inside the owned agent.

mod agent_bin;
mod discover;

pub use agent_bin::{CODEX_ACP_BIN, CODEX_ACP_ENV, codex_acp_launch, resolve_codex_acp_binary};

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
const HARNESS: &str = "openai-codex";

/// Preferred model for one-shot title / reply-suggest calls (cheap/fast Codex tier).
const TITLE_MODEL: &str = "gpt-5.4-mini";

/// [`Provider`] over the owned Codex ACP agent (`duckchat-codex-acp`). Models
/// are discovered once from the agent's ACP `initialize` handshake and cached
/// for the provider instance lifetime.
#[derive(Clone)]
pub struct OpenaiCodexProvider {
    launch: AgentLaunch,
    models: OnceLock<Vec<ModelInfo>>,
}

impl OpenaiCodexProvider {
    pub fn new() -> Self {
        Self {
            launch: codex_acp_launch(),
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

impl Default for OpenaiCodexProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Map a handshake-advertised model onto neutral [`ModelInfo`] for Codex.
fn to_model_info(m: AcpModel) -> ModelInfo {
    ModelInfo {
        harness: HARNESS.to_string(),
        id: m.id.clone(),
        display: humanize_display(&m.id, &m.name),
        context_window: m.context_window,
    }
}

/// Prefer a human advertised name; otherwise light-prettify the id.
fn humanize_display(id: &str, advertised: &str) -> String {
    let advertised = advertised.trim();
    if !advertised.is_empty() && advertised != id {
        return advertised.to_string();
    }
    // gpt-5.4-mini → Gpt 5.4 Mini-ish light prettify
    id.split(['-', '_'])
        .filter(|s| !s.is_empty())
        .map(|part| {
            if part.chars().all(|c| c.is_ascii_digit() || c == '.') {
                part.to_string()
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

#[async_trait]
impl Provider for OpenaiCodexProvider {
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
        // Host may pass a catalog-resolved id; default preferred is TITLE_MODEL.
        let preferred = preferred_model.or_else(|| Some(TITLE_MODEL.to_string()));
        Box::new(AcpOneshotRuntime::with_preferred_model(
            self.launch.clone(),
            working_dir,
            preferred,
        ))
    }

    async fn title_summary(&self, req: TitleRequest, working_dir: &Path) -> Result<String, Error> {
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
    use crate::acp::{AgentLaunch, pick_oneshot_model};
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
        install_fake_acp_agent_with_models(
            dir,
            r#"[{"modelId": "gpt-5.4-mini", "name": "GPT-5.4-Mini"}]"#,
        )
    }

    fn install_fake_acp_agent_with_models(dir: &TempDir, models_json: &str) -> PathBuf {
        let path = dir.path().join("fake-codex-acp");
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
        respond(req_id, {{"sessionId": "codex-thread-1"}})
    elif method == "session/load":
        respond(req_id, {{"sessionId": msg.get("params", {{}}).get("sessionId", "loaded")}})
    elif method == "session/prompt":
        send({{
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {{
                "sessionId": "codex-thread-1",
                "update": {{
                    "sessionUpdate": "agent_message_chunk",
                    "content": {{"type": "text", "text": "from-owned-codex-agent"}},
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

    fn acp_model(id: &str) -> AcpModel {
        AcpModel {
            id: id.to_string(),
            name: format!("{id} display"),
            context_window: None,
        }
    }

    /// @spec harness/openai-codex Owned ACP agent over official Codex: A Codex turn is driven through the owned ACP agent process
    #[tokio::test]
    async fn codex_turn_driven_through_owned_acp_agent() {
        let tmp = TempDir::new().unwrap();
        let provider = {
            let _guard = ENV_LOCK.lock().unwrap();
            let agent = install_fake_acp_agent(&tmp);

            // Launch is the owned agent binary — not an in-host App Server client.
            let launch = AgentLaunch::new({
                let agent = agent.clone();
                move || Command::new(&agent)
            });
            let provider = OpenaiCodexProvider::with_launch(launch);

            let cmd = codex_acp_launch().command();
            let program = cmd.as_std().get_program().to_string_lossy().into_owned();
            let args: Vec<String> = cmd
                .as_std()
                .get_args()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
            assert!(
                program.contains(CODEX_ACP_BIN) || args.iter().any(|a| a.contains(CODEX_ACP_BIN)),
                "default Codex launch must target {CODEX_ACP_BIN}, got program={program} args={args:?}"
            );
            // Host must not speak App Server methods itself.
            assert!(
                !args
                    .iter()
                    .any(|a| a == "app-server" || a.contains("thread/start")),
                "host must not drive Codex via in-host App Server: {args:?}"
            );
            provider
        };

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
        assert_eq!(outcome.session_id, "codex-thread-1");

        let mut saw_content = false;
        while let Ok(ev) = rx.try_recv() {
            if let AgentEvent::ContentDelta { text } = ev
                && text.contains("from-owned-codex-agent")
            {
                saw_content = true;
            }
        }
        assert!(
            saw_content,
            "host ACP client must receive profile content from the owned agent"
        );
    }

    /// @spec harness/openai-codex Model discovery and oneshot preference: Discovered models are tagged with the openai-codex harness
    #[test]
    fn discovered_models_tagged_openai_codex() {
        let tmp = TempDir::new().unwrap();
        let agent = install_fake_acp_agent_with_models(
            &tmp,
            r#"[
                {"modelId": "gpt-5.4", "name": "GPT-5.4"},
                {"modelId": "gpt-5.4-mini", "name": "GPT-5.4-Mini"}
            ]"#,
        );
        let provider = OpenaiCodexProvider::with_launch(AgentLaunch::new({
            let agent = agent.clone();
            move || Command::new(&agent)
        }));

        let listed = provider.list_models();
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().all(|m| m.harness == "openai-codex"));
        assert_eq!(listed[0].id, "gpt-5.4");
        assert_eq!(listed[1].id, "gpt-5.4-mini");
    }

    /// @spec harness/openai-codex Model discovery and oneshot preference: Each listed model carries a display name
    #[test]
    fn each_listed_model_carries_a_display_name() {
        let handshake = vec![
            AcpModel {
                id: "gpt-5.4".into(),
                name: "GPT-5.4".into(),
                context_window: None,
            },
            AcpModel {
                id: "gpt-5.4-mini".into(),
                name: "gpt-5.4-mini".into(), // same as id → humanize
                context_window: None,
            },
        ];
        let listed: Vec<ModelInfo> = handshake.into_iter().map(to_model_info).collect();
        assert!(listed.iter().all(|m| !m.display.is_empty()));
        assert_eq!(listed[0].display, "GPT-5.4");
        assert!(!listed[1].display.is_empty());
    }

    /// @spec harness/openai-codex Model discovery and oneshot preference: Preferred oneshot model is selected when advertised
    #[test]
    fn preferred_oneshot_model_is_selected_when_advertised() {
        let models = vec![
            acp_model("gpt-5.4"),
            acp_model("gpt-5.6-sol"),
            acp_model(TITLE_MODEL),
        ];
        let selected = pick_oneshot_model(Some(TITLE_MODEL), &models);
        assert_eq!(selected.as_deref(), Some(TITLE_MODEL));
    }

    /// @spec harness/openai-codex Model discovery and oneshot preference: Oneshot model falls back when preferred is absent
    #[test]
    fn oneshot_model_falls_back_when_preferred_is_absent() {
        let models = vec![acp_model("gpt-5.4"), acp_model("gpt-5.6-sol")];
        let selected = pick_oneshot_model(Some(TITLE_MODEL), &models);
        // Fall back to another advertised model rather than failing.
        assert!(selected.is_some());
        assert_ne!(selected.as_deref(), Some(TITLE_MODEL));
        assert!(
            selected.as_deref() == Some("gpt-5.4") || selected.as_deref() == Some("gpt-5.6-sol")
        );
    }

    /// @spec harness/openai-codex Graceful unavailability: A missing agent or backend yields no models and a turn error
    #[tokio::test]
    async fn missing_agent_yields_no_models_and_turn_error() {
        let provider = OpenaiCodexProvider::with_launch(AgentLaunch::new(|| {
            Command::new("/nonexistent/duckchat-codex-acp-does-not-exist")
        }));

        let listed = provider.list_models();
        assert!(
            listed.is_empty(),
            "expected empty model list, got {listed:?}"
        );

        let mut rt = provider.open_main_runtime(std::path::Path::new("/tmp"));
        let (tx, _rx) = mpsc::channel(8);
        let req = TurnRequest::new("hello", std::env::temp_dir());
        let outcome = rt
            .run_turn(
                req,
                tx,
                CancelToken::new(),
                crate::event::PendingUserChoices::shared(),
            )
            .await;
        assert!(
            matches!(outcome, Err(Error::Spawn(_))),
            "expected typed Spawn error, got {outcome:?}"
        );
    }

    #[test]
    fn provider_id_and_title_model_constants() {
        let provider = OpenaiCodexProvider::new();
        assert_eq!(provider.id(), "openai-codex");
        assert_eq!(TITLE_MODEL, "gpt-5.4-mini");
        assert!(provider.capabilities().streaming);
        assert!(provider.capabilities().slash_commands);
    }
}
