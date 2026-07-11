use std::fmt;
use std::path::Path;

use async_trait::async_trait;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use tokio::sync::mpsc;

use crate::cancel::CancelToken;
use crate::error::Error;
use crate::event::AgentEvent;
use crate::request::{ReplySuggestionRequest, TitleRequest, TurnOutcome, TurnRequest};
use crate::runtime::{MainRuntime, OneshotRuntime};

/// A source of agent turns. Implementations may spawn subprocesses (Claude
/// Code, opencode) or call LLM APIs directly.
///
/// Discovery (`list_models`, `list_commands`) stays on the provider. Live work
/// goes through [`Self::open_main_runtime`] / [`Self::open_oneshot_runtime`].
/// The free-standing `run_turn` / `title_summary` / `reply_suggestions`
/// methods are thin transitional wrappers for callers that have not yet moved
/// onto the handle/runtime path.
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

    /// Open a main-turn runtime bound to `working_dir`.
    fn open_main_runtime(&self, working_dir: &Path) -> Box<dyn MainRuntime>;

    /// Open a oneshot (title / reply-suggest) runtime bound to `working_dir`.
    fn open_oneshot_runtime(&self, working_dir: &Path) -> Box<dyn OneshotRuntime>;

    /// Transitional: open a cold main runtime and run one turn.
    async fn run_turn(
        &self,
        req: TurnRequest,
        events: mpsc::Sender<AgentEvent>,
        cancel: CancelToken,
    ) -> Result<TurnOutcome, Error> {
        let mut rt = self.open_main_runtime(&req.working_dir);
        rt.run_turn(
            req,
            events,
            cancel,
            crate::event::PendingUserChoices::shared(),
        )
        .await
    }

    /// Transitional: open a cold oneshot runtime and summarise a title.
    /// Harness-specific prompt framing lives in each provider override when
    /// needed; default is not provided because framing differs.
    async fn title_summary(
        &self,
        req: TitleRequest,
        working_dir: &std::path::Path,
    ) -> Result<String, Error>;

    /// Transitional: open a cold oneshot runtime and suggest replies.
    async fn reply_suggestions(
        &self,
        req: ReplySuggestionRequest,
        working_dir: &std::path::Path,
    ) -> Result<Vec<String>, Error>;
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
    /// The harness that owns this model, e.g. `"claude-code"` | `"grok"`. Keeps
    /// aggregated model lists unambiguous when more than one harness contributes.
    pub harness: String,
    pub id: String,
    pub display: String,
    /// Context window in tokens, when the harness reports one. Drives the usage
    /// meter denominator; `None` when the harness exposes no such figure.
    pub context_window: Option<usize>,
}

/// The persisted unit of model choice: a harness id paired with a model id.
///
/// Deserialization accepts either the struct form (`{ harness, model }`) or a
/// legacy bare model-id string, which loads under the `claude-code` harness via
/// [`ModelRef::parse_legacy`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelRef {
    pub harness: String,
    pub model: String,
}

impl ModelRef {
    pub fn new(harness: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            harness: harness.into(),
            model: model.into(),
        }
    }

    /// Map a bare model-id string (the legacy persisted form, before harnesses
    /// existed) to the `claude-code` harness.
    pub fn parse_legacy(raw: &str) -> Self {
        Self {
            harness: "claude-code".to_string(),
            model: raw.to_string(),
        }
    }
}

impl<'de> Deserialize<'de> for ModelRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ModelRefVisitor;

        impl<'de> Visitor<'de> for ModelRefVisitor {
            type Value = ModelRef;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a bare model-id string or a { harness, model } map")
            }

            fn visit_str<E>(self, v: &str) -> Result<ModelRef, E>
            where
                E: de::Error,
            {
                Ok(ModelRef::parse_legacy(v))
            }

            fn visit_map<A>(self, mut map: A) -> Result<ModelRef, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut harness: Option<String> = None;
                let mut model: Option<String> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "harness" => harness = Some(map.next_value()?),
                        "model" => model = Some(map.next_value()?),
                        _ => {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                Ok(ModelRef {
                    harness: harness.ok_or_else(|| de::Error::missing_field("harness"))?,
                    model: model.ok_or_else(|| de::Error::missing_field("model"))?,
                })
            }
        }

        deserializer.deserialize_any(ModelRefVisitor)
    }
}

/// A slash command exposed to the chat input (`/review`, `/plan`, ...).
#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @spec harness/selection Harness-tagged model identity: A model choice round-trips its harness and model
    #[test]
    fn model_ref_round_trips_harness_and_model() {
        let chosen = ModelRef::new("grok", "grok-4.5");
        let persisted = serde_json::to_string(&chosen).unwrap();
        let loaded: ModelRef = serde_json::from_str(&persisted).unwrap();
        assert_eq!(loaded.harness, "grok");
        assert_eq!(loaded.model, "grok-4.5");
        assert_eq!(loaded, chosen);
    }

    /// @spec harness/selection Harness-tagged model identity: A legacy bare model id loads as the Claude harness
    #[test]
    fn legacy_bare_model_id_loads_as_claude_harness() {
        let loaded: ModelRef = serde_json::from_str("\"opus\"").unwrap();
        assert_eq!(loaded.harness, "claude-code");
        assert_eq!(loaded.model, "opus");
    }
}
