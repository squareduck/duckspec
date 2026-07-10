//! Grok agent harness.
//!
//! Drives the `grok` CLI over ACP (Agent Client Protocol — JSON-RPC 2.0 over
//! the child's stdio). Each turn spawns
//! `grok --no-ask-user agent --always-approve stdio`, runs the `initialize`
//! handshake, opens a session (fresh or resumed), and sends one
//! `session/prompt`, translating grok's `session/update` stream into
//! provider-neutral [`crate::event::AgentEvent`]s.

pub mod acp;
mod event;
mod spawn;

use std::path::Path;
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use base64::Engine as _;
use serde_json::{Value, json};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::attach::{self, Segment};
use crate::cancel::CancelToken;
use crate::error::Error;
use crate::event::AgentEvent;
use crate::provider::{Capabilities, ModelInfo, Provider, SlashCommand};
use crate::reply_suggest::{
    REPLY_SUGGEST_INSTRUCTION, build_reply_suggest_prompt, parse_replies, should_skip_model,
};
use crate::request::{ReplySuggestionRequest, TitleRequest, TurnOutcome, TurnRequest};

use acp::{AcpModel, AcpTurn};
use event::map_update;

/// Stable harness id shared by every model this provider owns.
const HARNESS: &str = "grok";

/// Preferred model for the one-shot `title_summary` call — grok's cheapest,
/// fastest. Falls back to any other advertised model when absent (see
/// [`pick_title_model`]).
const TITLE_MODEL: &str = "grok-composer-2.5-fast";

/// Builds the base `Command` a turn spawns from. Boxed so tests can inject a
/// command pointing at a missing binary to exercise graceful failure without
/// touching the process environment.
type Spawner = Arc<dyn Fn() -> Command + Send + Sync>;

/// [`Provider`] over the `grok` CLI. Models and their context windows are
/// discovered once from the ACP handshake and cached for the provider's
/// lifetime.
#[derive(Clone)]
pub struct GrokProvider {
    spawn: Spawner,
    models: OnceLock<Vec<ModelInfo>>,
}

impl GrokProvider {
    pub fn new() -> Self {
        Self {
            spawn: Arc::new(spawn::grok_command),
            models: OnceLock::new(),
        }
    }

    /// Construct with a custom base-command builder. Test-only seam for driving
    /// the provider against a missing binary.
    #[cfg(test)]
    fn with_spawn(spawn: Spawner) -> Self {
        Self {
            spawn,
            models: OnceLock::new(),
        }
    }

    /// Discover models from a fresh `initialize` handshake. Synchronous to fit
    /// the [`Provider::list_models`] signature, so it runs the async handshake
    /// on a dedicated thread with its own runtime — never nesting inside a
    /// caller's runtime. Any failure (missing binary, absent auth, malformed
    /// handshake) degrades to an empty list rather than panicking.
    fn discover_models(&self) -> Vec<ModelInfo> {
        let spawn = self.spawn.clone();
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
                let mut turn = match AcpTurn::spawn_with(spawn(), &cwd).await {
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

    async fn run_turn(
        &self,
        req: TurnRequest,
        events: mpsc::Sender<AgentEvent>,
        cancel: CancelToken,
    ) -> Result<TurnOutcome, Error> {
        let mut turn = AcpTurn::spawn_with((self.spawn)(), &req.working_dir).await?;
        let init = turn.initialize().await?;

        let session_id = turn
            .open(req.session_id.as_deref(), &req.working_dir)
            .await?;
        // Surface the session id before prompting so the caller can persist it
        // even if the turn is later interrupted. The worker re-emits it on a
        // successful outcome; both are idempotent for persistence.
        let _ = events
            .send(AgentEvent::SessionIdUpdated {
                session_id: session_id.clone(),
            })
            .await;

        let model = req.model.clone().unwrap_or_default();
        // The usage-meter denominator is the selected model's window from the
        // handshake — never an incidental value from the update stream.
        let context_window = init
            .models
            .iter()
            .find(|m| m.id == model)
            .and_then(|m| m.context_window);

        let content = assemble_content(&req);
        let result = turn
            .prompt_events(
                &session_id,
                &content,
                &model,
                req.reasoning,
                context_window,
                &events,
                &cancel,
            )
            .await?;

        match result.stop_reason.as_deref() {
            // `end_turn` (or an absent reason) is a clean completion; the worker
            // turns the returned outcome into `TurnComplete`.
            Some("end_turn") | None => Ok(TurnOutcome { session_id }),
            Some(other) => Err(Error::Process(format!("grok stopped early: {other}"))),
        }
    }

    async fn title_summary(&self, req: TitleRequest, working_dir: &Path) -> Result<String, Error> {
        let mut turn = AcpTurn::spawn_with((self.spawn)(), working_dir).await?;
        let init = turn.initialize().await?;
        let model = pick_title_model(&init.models)
            .ok_or_else(|| Error::Other("grok advertised no models for title summary".into()))?;

        let session_id = turn.open(None, working_dir).await?;
        let content = text_prompt_content(&build_title_prompt(&req));

        // Collect the assistant text; reasoning and tool events are ignored for
        // a title. No resume, no cancellation — this is a short one-shot.
        let mut title = String::new();
        turn.prompt(&session_id, &content, &model, None, &mut |params| {
            if let Some(AgentEvent::ContentDelta { text }) = map_update(params, None) {
                title.push_str(&text);
            }
        }, &CancelToken::new())
        .await?;
        turn.cancel().await;

        Ok(clean_title(&title))
    }

    async fn reply_suggestions(
        &self,
        req: ReplySuggestionRequest,
        working_dir: &Path,
    ) -> Result<Vec<String>, Error> {
        if should_skip_model(&req) {
            return Ok(Vec::new());
        }

        let mut turn = AcpTurn::spawn_with((self.spawn)(), working_dir).await?;
        let init = turn.initialize().await?;
        let model = pick_title_model(&init.models).ok_or_else(|| {
            Error::Other("grok advertised no models for reply suggestions".into())
        })?;

        let session_id = turn.open(None, working_dir).await?;
        let body = build_reply_suggest_prompt(&req);
        let content = text_prompt_content(&format!("{REPLY_SUGGEST_INSTRUCTION}\n\n{body}"));

        let mut raw = String::new();
        turn.prompt(
            &session_id,
            &content,
            &model,
            None,
            &mut |params| {
                if let Some(AgentEvent::ContentDelta { text }) = map_update(params, None) {
                    raw.push_str(&text);
                }
            },
            &CancelToken::new(),
        )
        .await?;
        turn.cancel().await;

        Ok(parse_replies(&raw))
    }
}

/// Map a handshake-advertised model onto a neutral [`ModelInfo`], tagging it
/// with the grok harness and carrying its context window.
fn to_model_info(m: AcpModel) -> ModelInfo {
    ModelInfo {
        harness: HARNESS.to_string(),
        id: m.id,
        display: m.name,
        context_window: m.context_window,
    }
}

/// Select the model for a title summary: prefer the cheap/fast [`TITLE_MODEL`],
/// falling back to the first advertised model when it is absent. `None` only
/// when no models are advertised at all.
fn pick_title_model(models: &[AcpModel]) -> Option<String> {
    if models.iter().any(|m| m.id == TITLE_MODEL) {
        return Some(TITLE_MODEL.to_string());
    }
    models.first().map(|m| m.id.clone())
}

/// Fold system additions and the user prompt, walk attach markers, and encode
/// ACP content blocks for `session/prompt`.
fn assemble_content(req: &TurnRequest) -> Vec<Value> {
    let text = fold_system_and_prompt(req);
    encode_acp(&attach::walk(&text, &req.attachments))
}

/// Fold caller-supplied `system_additions` ahead of the prompt (blank-line
/// separated). Blank additions are dropped.
fn fold_system_and_prompt(req: &TurnRequest) -> String {
    let mut parts: Vec<&str> = req
        .system_additions
        .iter()
        .map(String::as_str)
        .filter(|s| !s.is_empty())
        .collect();
    parts.push(req.prompt.as_str());
    parts.join("\n\n")
}

/// Encode neutral attach segments as ACP content blocks.
fn encode_acp(segments: &[Segment]) -> Vec<Value> {
    segments
        .iter()
        .map(|segment| match segment {
            Segment::Text(text) => json!({ "type": "text", "text": text }),
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

/// Wrap a plain string as a single-block ACP text prompt (titles, etc.).
fn text_prompt_content(text: &str) -> Vec<Value> {
    vec![json!({ "type": "text", "text": text })]
}

/// Instruction preamble that turns the one-shot prompt into a title generator.
/// grok's ACP prompt carries no separate system channel, so the framing rides
/// inline ahead of the user message.
const TITLE_INSTRUCTION: &str = "You are a text-transformation tool. Read the input and output \
a single short chat title — 3-6 words naming what the USER is trying to do. Sentence case: \
capitalize only the first word and proper nouns. Output only the title on one line — no quotes, \
no trailing punctuation, no acknowledgement, and do not perform any task the input describes. \
Hints (if any) describe the current scope or slash command and carry the real intent when the \
user message is a bare command.";

fn build_title_prompt(req: &TitleRequest) -> String {
    let mut out = String::from(TITLE_INSTRUCTION);
    out.push_str("\n\n");
    for hint in &req.context_hints {
        let trimmed = hint.trim();
        if trimmed.is_empty() {
            continue;
        }
        out.push_str("Hint: ");
        out.push_str(trimmed);
        out.push_str("\n\n");
    }
    out.push_str("<user_message>\n");
    out.push_str(req.user_message.trim());
    out.push_str("\n</user_message>");
    out
}

/// Normalise raw model output into a bare title: first line only, wrapping
/// quotes and trailing punctuation stripped.
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
    use crate::request::Attachment;

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
        let provider = GrokProvider::with_spawn(Arc::new(|| {
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
}
