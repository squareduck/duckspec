//! Grok agent harness.
//!
//! Thin provider over the shared [`crate::acp`] client. Builds a native
//! `grok agent --always-approve stdio` [`AgentLaunch`] (login-shell wrap) and
//! opens shared main/oneshot runtimes. Structured questions are enabled; tool
//! execution stays auto-approved. Model discovery, attach encoding helpers, and
//! title/reply prompts stay harness-local.

mod spawn;

use std::path::Path;
use std::sync::OnceLock;

use async_trait::async_trait;

use crate::acp::{AcpMainRuntime, AcpOneshotRuntime, AcpTurn, AgentLaunch};
use crate::error::Error;
use crate::provider::{Capabilities, ModelInfo, Provider, SlashCommand};
use crate::reply_suggest::{
    REPLY_SUGGEST_INSTRUCTION, build_reply_suggest_prompt, parse_replies, should_skip_model,
};
use crate::request::{ReplySuggestionRequest, TitleRequest};
use crate::runtime::{MainRuntime, OneshotKind, OneshotRuntime};
use crate::title::{build_title_prompt, clean_title};

/// Stable harness id shared by every model this provider owns.
const HARNESS: &str = "grok";

/// Default preferred oneshot model (title summary / reply suggest) when present
/// in the advertised catalog. Falls back to another available model when absent
/// (see [`pick_title_model`]).
const TITLE_MODEL: &str = "grok-composer-2.5-fast";

/// [`Provider`] over the `grok` CLI. Models and their context windows are
/// discovered once from the ACP handshake and cached for the provider's
/// lifetime.
#[derive(Clone)]
pub struct GrokProvider {
    launch: AgentLaunch,
    models: OnceLock<Vec<ModelInfo>>,
}

impl GrokProvider {
    pub fn new() -> Self {
        Self {
            launch: grok_agent_launch(),
            models: OnceLock::new(),
        }
    }

    /// Construct with a custom launch. Test-only seam for driving the provider
    /// against a missing binary.
    #[cfg(test)]
    fn with_launch(launch: AgentLaunch) -> Self {
        Self {
            launch,
            models: OnceLock::new(),
        }
    }

    /// Discover models from a fresh `initialize` handshake. Synchronous to fit
    /// the [`Provider::list_models`] signature, so it runs the async handshake
    /// on a dedicated thread with its own runtime — never nesting inside a
    /// caller's runtime. Any failure (missing binary, absent auth, malformed
    /// handshake) degrades to an empty list rather than panicking.
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

impl Default for GrokProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for GrokProvider {
    fn id(&self) -> &str {
        HARNESS
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            streaming: true,
            tool_use: true,
            resume: true,
            reasoning: true,
            slash_commands: true,
        }
    }

    fn list_models(&self) -> Vec<ModelInfo> {
        self.models.get_or_init(|| self.discover_models()).clone()
    }

    fn list_commands(&self, project_root: &Path) -> Vec<SlashCommand> {
        // grok loads the same `.claude` skills/commands as Claude Code, so the
        // discovery is identical.
        crate::claude_code::discover_commands(project_root)
    }

    fn open_main_runtime(&self, working_dir: &Path) -> Box<dyn MainRuntime> {
        Box::new(AcpMainRuntime::new(self.launch.clone(), working_dir))
    }

    fn open_oneshot_runtime(
        &self,
        working_dir: &Path,
        preferred_model: Option<String>,
    ) -> Box<dyn OneshotRuntime> {
        // Host preferred when set; otherwise advertise-side default needles.
        Box::new(AcpOneshotRuntime::with_preferred_model(
            self.launch.clone(),
            working_dir,
            preferred_model,
        ))
    }

    async fn title_summary(&self, req: TitleRequest, working_dir: &Path) -> Result<String, Error> {
        // TITLE_MODEL needle matches full ids via pick_oneshot_model substring.
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

/// Build the native Grok ACP agent launch: login-shell wrap of
/// `grok agent --always-approve stdio`.
///
/// Structured questions are enabled (no `--no-ask-user`). Tool execution is still
/// auto-approved via `--always-approve`. Flags live on the launch (final argv);
/// the shared client does not append them.
pub fn grok_agent_launch() -> AgentLaunch {
    AgentLaunch::new(|| {
        let mut cmd = spawn::grok_command();
        cmd.arg("agent").arg("--always-approve").arg("stdio");
        cmd
    })
}

/// Map a handshake-advertised model onto a neutral [`ModelInfo`], tagging it
/// with the grok harness and carrying its context window and display name.
fn to_model_info(m: crate::acp::AcpModel) -> ModelInfo {
    ModelInfo {
        harness: HARNESS.to_string(),
        id: m.id.clone(),
        display: humanize_display(&m.id, &m.name),
        context_window: m.context_window,
    }
}

/// Prefer a human advertised name; otherwise light-prettify the bare id.
fn humanize_display(id: &str, advertised: &str) -> String {
    let advertised = advertised.trim();
    if !advertised.is_empty() && advertised != id {
        return advertised.to_string();
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::AcpModel;
    use crate::attach::{self, Segment};
    use crate::cancel::CancelToken;
    use crate::request::{Attachment, TurnRequest};
    use base64::Engine as _;
    use serde_json::{Value, json};
    use tokio::process::Command;
    use tokio::sync::mpsc;

    fn model(id: &str, window: Option<usize>) -> AcpModel {
        AcpModel {
            id: id.to_string(),
            name: format!("{id} display"),
            context_window: window,
        }
    }

    fn img_att(label: &str) -> Attachment {
        Attachment {
            label: label.to_string(),
            media_type: "image/png".to_string(),
            bytes: vec![1, 2, 3, 4],
        }
    }

    fn text_block(blocks: &[Value], i: usize) -> &str {
        blocks[i]["text"].as_str().unwrap_or("")
    }

    fn pick_title_model(models: &[AcpModel]) -> Option<String> {
        if models.iter().any(|m| m.id == TITLE_MODEL) {
            return Some(TITLE_MODEL.to_string());
        }
        models.first().map(|m| m.id.clone())
    }

    fn assemble_content(req: &TurnRequest) -> Vec<Value> {
        let text = {
            let mut parts: Vec<&str> = req
                .system_additions
                .iter()
                .map(String::as_str)
                .filter(|s| !s.is_empty())
                .collect();
            parts.push(req.prompt.as_str());
            parts.join("\n\n")
        };
        attach::walk(&text, &req.attachments)
            .into_iter()
            .map(|segment| match segment {
                Segment::Text(t) => json!({ "type": "text", "text": t }),
                Segment::Image { media_type, bytes } => {
                    let data = base64::engine::general_purpose::STANDARD.encode(bytes);
                    json!({
                        "type": "image",
                        "mimeType": media_type,
                        "data": data,
                    })
                }
            })
            .collect()
    }

    /// @spec harness/grok Prompt attachments: A resolved image attachment is sent as an ACP image block
    #[test]
    fn resolved_image_attachment_is_sent_as_acp_image_block() {
        let mut req = TurnRequest::new("see [clip.png](attach:a1)", std::env::temp_dir());
        req.attachments.insert("a1".to_string(), img_att("clip.png"));

        let blocks = assemble_content(&req);
        let image = blocks
            .iter()
            .find(|b| b["type"] == "image")
            .expect("image content block");
        assert_eq!(image["mimeType"], "image/png");
        assert_eq!(
            image["data"].as_str().unwrap(),
            base64::engine::general_purpose::STANDARD.encode([1, 2, 3, 4])
        );
        // ACP shape, not Anthropic's nested `source`.
        assert!(image.get("source").is_none());
    }

    /// @spec harness/grok Prompt attachments: Surrounding text is preserved as text blocks
    #[test]
    fn surrounding_text_is_preserved_as_text_blocks() {
        let mut req = TurnRequest::new(
            "before [clip.png](attach:a1) after",
            std::env::temp_dir(),
        );
        req.attachments.insert("a1".to_string(), img_att("clip.png"));

        let blocks = assemble_content(&req);
        assert_eq!(blocks.len(), 3);
        assert_eq!(text_block(&blocks, 0), "before ");
        assert_eq!(blocks[1]["type"], "image");
        assert_eq!(text_block(&blocks, 2), " after");
    }

    /// @spec harness/grok Prompt attachments: A non-image attachment is represented as text
    #[test]
    fn non_image_attachment_is_represented_as_text() {
        let mut req = TurnRequest::new("file [notes.txt](attach:f1) end", std::env::temp_dir());
        req.attachments.insert(
            "f1".to_string(),
            Attachment {
                label: "notes.txt".to_string(),
                media_type: "text/plain".to_string(),
                bytes: vec![9, 9],
            },
        );

        let blocks = assemble_content(&req);
        assert!(blocks.iter().all(|b| b["type"] != "image"));
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            text_block(&blocks, 0),
            "file [attachment: notes.txt (2 bytes)] end"
        );
    }

    /// @spec harness/grok Prompt attachments: An unresolved attach marker is left literal
    #[test]
    fn unresolved_attach_marker_is_left_literal() {
        let req = TurnRequest::new("see [thing](attach:missing)", std::env::temp_dir());
        let blocks = assemble_content(&req);
        assert_eq!(blocks.len(), 1);
        assert_eq!(text_block(&blocks, 0), "see [thing](attach:missing)");
    }

    /// @spec harness/grok Model discovery: Discovered models are tagged with the grok harness and a window
    #[test]
    fn discovered_models_tagged_with_harness_and_window() {
        let handshake = vec![
            model("grok-4.5", Some(256_000)),
            model("grok-composer-2.5-fast", Some(128_000)),
        ];

        let listed: Vec<ModelInfo> = handshake.into_iter().map(to_model_info).collect();

        // Each returned model is tagged with the grok harness.
        assert!(listed.iter().all(|m| m.harness == "grok"));
        // Each returned model carries a context window.
        assert!(listed.iter().all(|m| m.context_window.is_some()));
        assert_eq!(listed[0].id, "grok-4.5");
        assert_eq!(listed[0].context_window, Some(256_000));
    }

    /// @spec harness/grok Model discovery: Each listed model carries a display name
    #[test]
    fn each_listed_model_carries_a_display_name() {
        // GIVEN a grok handshake advertising its available models
        let handshake = vec![
            AcpModel {
                id: "grok-4.5".into(),
                name: "Grok 4.5".into(),
                context_window: Some(256_000),
            },
            AcpModel {
                id: "grok-composer-2.5-fast".into(),
                name: "grok-composer-2.5-fast".into(), // bare id → humanize
                context_window: Some(128_000),
            },
        ];

        // WHEN the harness lists models
        let listed: Vec<ModelInfo> = handshake.into_iter().map(to_model_info).collect();

        // THEN each returned model carries a non-empty display name
        assert!(listed.iter().all(|m| !m.display.is_empty()));
        assert_eq!(listed[0].display, "Grok 4.5");
        assert_eq!(listed[1].display, "Grok Composer 2.5 Fast");
    }

    /// @spec harness/grok Model discovery: Title model falls back when the preferred fast model is absent
    #[test]
    fn title_model_falls_back_when_preferred_absent() {
        // Preferred fast model absent → selects another available model.
        let without_fast = vec![model("grok-4.5", Some(256_000)), model("grok-3", Some(131_072))];
        assert_eq!(pick_title_model(&without_fast).as_deref(), Some("grok-4.5"));

        // When present, it is preferred.
        let with_fast = vec![
            model("grok-4.5", Some(256_000)),
            model(TITLE_MODEL, Some(128_000)),
        ];
        assert_eq!(pick_title_model(&with_fast).as_deref(), Some(TITLE_MODEL));
    }

    /// @spec harness/grok Graceful unavailability: A missing grok binary yields no models and a turn error
    #[tokio::test]
    async fn missing_binary_yields_no_models_and_turn_error() {
        let provider = GrokProvider::with_launch(AgentLaunch::new(|| {
            Command::new("/nonexistent/grok-does-not-exist")
        }));

        // Listing models degrades to an empty list.
        assert!(provider.list_models().is_empty());

        // Running a turn fails with a typed error rather than panicking.
        let (tx, _rx) = mpsc::channel(16);
        let req = TurnRequest::new("hello", std::env::temp_dir());
        let outcome = provider.run_turn(req, tx, CancelToken::new()).await;
        assert!(matches!(outcome, Err(Error::Spawn(_))));
    }

    /// @spec harness/grok Native Grok agent launch: A Grok turn spawns the native grok ACP agent
    #[test]
    fn grok_turn_spawns_native_grok_acp_agent() {
        // Final argv is the native grok CLI in agent stdio mode — no intermediate
        // owned proxy binary in the chain.
        let cmd = grok_agent_launch().command();
        let program = cmd.as_std().get_program().to_string_lossy().into_owned();
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();

        // Login-shell wrap around `grok` (not a duckchat-owned proxy).
        assert!(
            program.contains("sh") || program.ends_with("zsh") || program.ends_with("bash"),
            "expected login shell, got {program}"
        );
        assert!(
            args.iter().any(|a| a == "grok"),
            "launch must invoke the native grok binary: {args:?}"
        );
        assert!(
            !args.iter().any(|a| a.contains("duckchat-claude") || a.contains("grok-proxy")),
            "must not route through an intermediate Grok-only ACP proxy: {args:?}"
        );
        // Agent stdio mode flags on the final argv (client does not add them).
        let grok_pos = args.iter().position(|a| a == "grok").expect("grok in argv");
        let after: Vec<&str> = args[grok_pos + 1..].iter().map(String::as_str).collect();
        assert_eq!(
            after,
            ["agent", "--always-approve", "stdio"],
            "native grok ACP agent argv after binary"
        );
    }

    fn grok_argv_after_binary() -> Vec<String> {
        let cmd = grok_agent_launch().command();
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        let grok_pos = args.iter().position(|a| a == "grok").expect("grok in argv");
        args[grok_pos + 1..].to_vec()
    }

    // @spec harness/grok Structured questions enabled: Main launch does not pass no-ask-user
    #[test]
    fn main_launch_does_not_pass_no_ask_user() {
        let after = grok_argv_after_binary();
        assert!(
            !after.iter().any(|a| a == "--no-ask-user"),
            "main launch must allow structured questions: {after:?}"
        );
    }

    // @spec harness/grok Structured questions enabled: Main launch still auto-approves tool execution
    #[test]
    fn main_launch_still_auto_approves_tool_execution() {
        let after = grok_argv_after_binary();
        assert!(
            after.iter().any(|a| a == "--always-approve"),
            "main launch must keep always-approve: {after:?}"
        );
    }

    // @spec harness/grok Question wire mapping: An ask-user extension request is exposed as a host user choice
    #[test]
    fn an_ask_user_extension_request_is_exposed_as_a_host_user_choice() {
        // Live capture method name (leading underscore) and unprefixed alias.
        assert!(crate::acp::turn_ask_user::is_ask_user_method(
            crate::acp::turn_ask_user::ASK_USER_METHOD
        ));
        assert!(crate::acp::turn_ask_user::is_ask_user_method(
            crate::acp::turn_ask_user::ASK_USER_METHOD_ALIAS
        ));
        assert!(!crate::acp::turn_ask_user::is_ask_user_method(
            "session/request_permission"
        ));

        // Decode path used by AcpTurn when classifying ask-user methods.
        let params = json!({
            "sessionId": "s1",
            "toolCallId": "tc1",
            "mode": "single",
            "questions": [{
                "question": "Ship?",
                "options": [
                    { "label": "Yes", "description": "go" },
                    { "label": "No", "description": "hold" }
                ]
            }]
        });
        let (prompt, options) = crate::acp::turn_ask_user::decode_options(&params);
        assert_eq!(prompt.as_deref(), Some("Ship?"));
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].label, "Yes");
        assert_eq!(options[1].id, "No"); // id defaults to label
    }

    // @spec harness/grok Question wire mapping: A host selection completes with an accepted questionnaire response
    #[test]
    fn a_host_selection_completes_with_an_accepted_questionnaire_response() {
        let result = crate::acp::turn_ask_user::encode_selected("Ship?", "Yes");
        // Live-proven flat outcome tag (not externally tagged Accepted).
        assert_eq!(result["outcome"], "accepted", "result={result}");
        assert_eq!(result["answers"]["Ship?"], "Yes");
        assert!(result["partial_answers"].is_null());
    }

    // @spec harness/grok Question wire mapping: A host cancel completes with a skip-interview response
    #[test]
    fn a_host_cancel_completes_with_a_skip_interview_response() {
        let result = crate::acp::turn_ask_user::encode_cancelled();
        assert_eq!(
            result["outcome"], "skip_interview",
            "cancel must be skip_interview: {result}"
        );
        assert!(result.get("answers").is_none());
    }

    // @spec harness/grok Question wire mapping: Host custom freeform answer completes with an accepted free-text answer
    #[test]
    fn host_custom_freeform_answer_completes_with_an_accepted_free_text_answer() {
        let free = "something else";
        let result = crate::acp::turn_ask_user::encode_selected("Ship?", free);
        assert_eq!(result["outcome"], "accepted", "result={result}");
        assert_eq!(result["answers"]["Ship?"], free);
        assert!(result["partial_answers"].is_null());
        assert_ne!(result["outcome"], "skip_interview");
    }
}
