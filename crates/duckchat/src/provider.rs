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

    /// Suggest 0–3 short user replies for an empty chat composer after an
    /// agent turn. Expected to use the provider's cheapest/fastest model
    /// (same pick as [`Self::title_summary`]). Implementations should not
    /// invoke tools or resume a prior session. Empty
    /// `req.assistant_message` short-circuits to an empty list without a
    /// model call.
    ///
    /// Return values are already parsed reply texts (not raw model output).
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
