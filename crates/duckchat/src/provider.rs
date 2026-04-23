use std::path::Path;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::cancel::CancelToken;
use crate::error::Error;
use crate::event::AgentEvent;
use crate::request::{TitleRequest, TurnOutcome, TurnRequest};

/// A source of agent turns. Implementations may spawn subprocesses (Claude
/// Code, opencode) or call LLM APIs directly.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Stable identifier, e.g. `"claude-code"`.
    fn id(&self) -> &str;

    /// What this provider supports.
    fn capabilities(&self) -> Capabilities;

    /// Models the provider knows about. Synchronous because the current
    /// provider discovers these from local config, not a network call.
    fn list_models(&self) -> Vec<ModelInfo> {
        Vec::new()
    }

    /// Slash commands (or other user-triggerable presets) discovered by the
    /// provider, scoped to `project_root`.
    fn list_commands(&self, project_root: &Path) -> Vec<SlashCommand>;

    /// Run one prompt turn, emitting events into `events` until the turn
    /// completes, errors, or is cancelled. Returns the session id to persist.
    async fn run_turn(
        &self,
        req: TurnRequest,
        events: mpsc::Sender<AgentEvent>,
        cancel: CancelToken,
    ) -> Result<TurnOutcome, Error>;

    /// Summarise a single-turn exchange as a short session title. Expected to
    /// use the provider's cheapest/fastest model — this is called once per
    /// new chat as soon as the first assistant reply lands, so latency and
    /// token cost both matter.
    ///
    /// `req.context_hints` are arbitrary lines the caller wants the
    /// summariser to consider (e.g. "user is implementing step foo.md").
    /// Returns a plain-text title (trimmed, no quotes, a handful of words).
    /// Implementations should not invoke tools or resume a prior session.
    async fn title_summary(
        &self,
        req: TitleRequest,
        working_dir: &std::path::Path,
    ) -> Result<String, Error>;
}

#[derive(Debug, Clone, Default)]
pub struct Capabilities {
    pub streaming: bool,
    pub tool_use: bool,
    pub resume: bool,
    pub reasoning: bool,
    pub slash_commands: bool,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub display: String,
}

/// A slash command exposed to the chat input (`/review`, `/plan`, ...).
#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
}
