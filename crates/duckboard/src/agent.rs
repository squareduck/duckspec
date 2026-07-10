//! iced subscription adapter around `duckchat`.
//!
//! The real agent harness lives in the `duckchat` crate. This module wraps it
//! for iced: each live chat session gets a `Subscription` that spawns a
//! `duckchat` worker, forwards provider events, and emits duckboard-specific
//! `Ready` / `CommandsAvailable` / `ProcessExited` bookends.

use std::path::PathBuf;
use std::sync::OnceLock;

use iced::Subscription;
use tokio::sync::mpsc;

pub use duckchat::{AgentHandle, ModelInfo, SlashCommand};

use duckchat::claude_code::ClaudeCodeProvider;
use duckchat::grok::GrokProvider;

/// Shared `GrokProvider` reused across `available_models` calls. `GrokProvider`
/// discovers its models by spawning `grok agent stdio` and caches them for the
/// instance's lifetime, so a fresh provider per aggregation would re-spawn grok
/// on every model-list read. Holding one instance memoizes that handshake.
fn grok_provider() -> &'static GrokProvider {
    static GROK: OnceLock<GrokProvider> = OnceLock::new();
    GROK.get_or_init(GrokProvider::new)
}

/// Models offered for the chat/project model picker, aggregated across every
/// registered harness. Claude Code's list is static and cheap; grok's is read
/// from the memoized provider above. When grok is absent its list is empty, so
/// aggregation stays panic-free with only the Claude models.
pub fn available_models() -> Vec<ModelInfo> {
    use duckchat::Provider;
    aggregate_models([
        ClaudeCodeProvider::new().list_models(),
        grok_provider().list_models(),
    ])
}

/// Flatten each registered harness's model list into a single picker list. The
/// union spans every harness that offered models, each entry still carrying its
/// own `harness` tag so dispatch stays unambiguous.
fn aggregate_models(per_harness: impl IntoIterator<Item = Vec<ModelInfo>>) -> Vec<ModelInfo> {
    per_harness.into_iter().flatten().collect()
}

/// The registered harness a turn dispatches to, chosen from the model's harness
/// id. Unknown or legacy ids fall back to Claude Code — the original single
/// backend, and the harness legacy bare-string pins load as. This is the single
/// source of truth the `agent_stream` and title-summary dispatch match on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Harness {
    ClaudeCode,
    Grok,
}

impl Harness {
    fn dispatch(harness: &str) -> Self {
        match harness {
            "grok" => Harness::Grok,
            _ => Harness::ClaudeCode,
        }
    }
}

// ── Duckboard-level event enum ──────────────────────────────────────────────

/// Events routed into the iced update loop. Wraps `duckchat::AgentEvent`
/// plus the subscription-lifecycle events duckboard needs (`Ready`,
/// `CommandsAvailable`, `ProcessExited`).
#[derive(Debug, Clone)]
pub enum AgentEvent {
    Ready(AgentHandle),
    CommandsAvailable(Vec<SlashCommand>),
    ContentDelta {
        text: String,
    },
    /// Streaming reasoning/thinking text, distinct from answer content. Only the
    /// grok harness emits this; the Claude path never does.
    ReasoningDelta {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    ToolResult {
        id: String,
        name: String,
        output: String,
    },
    UsageUpdate {
        input_tokens: usize,
        output_tokens: usize,
    },
    SessionIdUpdated {
        session_id: String,
    },
    /// Stored resume id is dead; UI should clear it and re-dispatch with history.
    SessionNotFound,
    TurnComplete,
    Error(String),
    ProcessExited,
}

// ── Subscription ────────────────────────────────────────────────────────────

/// Create a subscription that manages one agent chat session.
///
/// `key` is an opaque routing token that gets echoed back with every event so
/// the caller can demultiplex when several sessions run in parallel. `harness`
/// selects the provider that runs the session's turns; it is folded into the
/// subscription's identity so switching harness respawns the worker on the new
/// backend (a grok session id can't `session/load` under Claude, and vice
/// versa).
pub fn agent_subscription(
    key: String,
    project_root: PathBuf,
    harness: String,
) -> Subscription<(String, AgentEvent)> {
    Subscription::run_with((key, project_root.clone(), harness), |(key, root, harness)| {
        use iced::futures::StreamExt;
        let key = key.clone();
        agent_stream(root.clone(), harness.clone()).map(move |e| (key.clone(), e))
    })
}

fn agent_stream(
    project_root: PathBuf,
    harness: String,
) -> impl iced::futures::Stream<Item = AgentEvent> {
    iced::stream::channel(
        256,
        move |sender: iced::futures::channel::mpsc::Sender<AgentEvent>| async move {
            // Harness dispatch: the session's harness names the provider that
            // runs its turns. `spawn_worker<P>` is monomorphized per arm, so the
            // driver is generic over the concrete provider — no trait object.
            match Harness::dispatch(&harness) {
                Harness::Grok => drive_provider(GrokProvider::new(), project_root, sender).await,
                Harness::ClaudeCode => {
                    drive_provider(ClaudeCodeProvider::new(), project_root, sender).await
                }
            }
        },
    )
}

/// Spawn a worker for `provider`, forward its command list and mapped events to
/// `sender`, and emit the `Ready` / `ProcessExited` bookends. Generic over the
/// provider so each harness arm monomorphizes its own driver.
async fn drive_provider<P: duckchat::Provider + 'static>(
    provider: P,
    project_root: PathBuf,
    mut sender: iced::futures::channel::mpsc::Sender<AgentEvent>,
) {
    use iced::futures::SinkExt;

    // Same cwd normalization grok uses for session keys — keep the worker's
    // working_dir and ACP `cwd` on one stable form.
    let project_root = duckchat::normalize_cwd(&project_root);

    let commands = provider.list_commands(&project_root);

    let (ev_tx, mut ev_rx) = mpsc::channel::<duckchat::AgentEvent>(256);
    let handle = duckchat::spawn_worker(provider, project_root.clone(), ev_tx);

    if sender.send(AgentEvent::Ready(handle)).await.is_err() {
        return;
    }
    if !commands.is_empty()
        && sender
            .send(AgentEvent::CommandsAvailable(commands))
            .await
            .is_err()
    {
        return;
    }

    while let Some(core) = ev_rx.recv().await {
        let mapped = match core {
            duckchat::AgentEvent::ContentDelta { text } => AgentEvent::ContentDelta { text },
            // grok emits reasoning; surface it as a distinct event instead of
            // dropping it (the Claude path never reaches here).
            duckchat::AgentEvent::ReasoningDelta { text } => AgentEvent::ReasoningDelta { text },
            duckchat::AgentEvent::ToolUse { id, name, input } => {
                AgentEvent::ToolUse { id, name, input }
            }
            duckchat::AgentEvent::ToolResult { id, name, output } => {
                AgentEvent::ToolResult { id, name, output }
            }
            // The observed-model readout was dropped from the UI, so a bare
            // model report has nothing to update — skip it rather than emit a
            // no-op usage event.
            duckchat::AgentEvent::ModelUpdate { .. } => continue,
            duckchat::AgentEvent::UsageUpdate(usage) => AgentEvent::UsageUpdate {
                input_tokens: usage.input_tokens.unwrap_or(0),
                output_tokens: usage.output_tokens.unwrap_or(0),
            },
            duckchat::AgentEvent::SessionIdUpdated { session_id } => {
                AgentEvent::SessionIdUpdated { session_id }
            }
            duckchat::AgentEvent::SessionNotFound => AgentEvent::SessionNotFound,
            duckchat::AgentEvent::TurnComplete => AgentEvent::TurnComplete,
            duckchat::AgentEvent::Error(msg) => AgentEvent::Error(msg),
        };
        if sender.send(mapped).await.is_err() {
            break;
        }
    }

    let _ = sender.send(AgentEvent::ProcessExited).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use duckchat::Provider;

    // @spec harness/selection Harness dispatch: A model's harness selects the provider that runs its turn
    #[test]
    fn harness_selects_provider() {
        // `Harness::dispatch` is the routing `agent_stream` and the
        // title-summary site match on. Each arm builds the provider whose
        // `Provider::id()` equals the harness that selected it, so a model's
        // harness id names the backend that runs its turn.
        assert_eq!(Harness::dispatch("grok"), Harness::Grok);
        assert_eq!(GrokProvider::new().id(), "grok");

        assert_eq!(Harness::dispatch("claude-code"), Harness::ClaudeCode);
        assert_eq!(ClaudeCodeProvider::new().id(), "claude-code");

        // Unknown / legacy ids fall back to Claude Code, never panic.
        assert_eq!(Harness::dispatch("legacy-bare-id"), Harness::ClaudeCode);
    }

    // @spec harness/selection Harness dispatch: The offered models span every registered harness
    #[test]
    fn offered_models_span_every_harness() {
        // Two registered harnesses, each offering a model. `aggregate_models`
        // is the union that `available_models` builds; the offered list must
        // include a model from every harness.
        let claude = vec![ModelInfo {
            harness: "claude-code".to_string(),
            id: "opus".to_string(),
            display: "Opus".to_string(),
            context_window: None,
        }];
        let grok = vec![ModelInfo {
            harness: "grok".to_string(),
            id: "grok-4.5".to_string(),
            display: "Grok 4.5".to_string(),
            context_window: Some(256_000),
        }];

        let offered = aggregate_models([claude, grok]);
        let harnesses: std::collections::HashSet<&str> =
            offered.iter().map(|m| m.harness.as_str()).collect();

        assert!(harnesses.contains("claude-code"));
        assert!(harnesses.contains("grok"));
    }
}
