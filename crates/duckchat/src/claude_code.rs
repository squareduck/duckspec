//! Provider implementation that drives the `claude` CLI.
//!
//! Spawns `claude -p --output-format stream-json` per turn, parses its ndjson
//! protocol, and forwards provider-neutral events. Multi-turn continuity is
//! achieved via `--resume <session_id>`.

mod discover;
mod protocol;
mod run;
mod title;

use std::path::Path;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::cancel::CancelToken;
use crate::error::Error;
use crate::event::AgentEvent;
use crate::provider::{Capabilities, ModelInfo, Provider, SlashCommand};
use crate::request::{TitleRequest, TurnOutcome, TurnRequest};

/// Model used for the one-shot `title_summary` call. Cheap, fast, plenty good
/// for summarising a single exchange into a handful of words.
pub const TITLE_MODEL: &str = "claude-haiku-4-5";

#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeProvider;

impl ClaudeCodeProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Provider for ClaudeCodeProvider {
    fn id(&self) -> &str {
        "claude-code"
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
        // Claude Code exposes model selection through its own `/model` slash
        // command; we don't duplicate the list here.
        Vec::new()
    }

    fn list_commands(&self, project_root: &Path) -> Vec<SlashCommand> {
        discover::discover_commands(project_root)
    }

    async fn run_turn(
        &self,
        req: TurnRequest,
        events: mpsc::Sender<AgentEvent>,
        cancel: CancelToken,
    ) -> Result<TurnOutcome, Error> {
        run::run_turn(req, events, cancel).await
    }

    async fn title_summary(
        &self,
        req: TitleRequest,
        working_dir: &Path,
    ) -> Result<String, Error> {
        title::title_summary(req, working_dir).await
    }
}
