//! iced subscription adapter around `duckchat`.
//!
//! The real agent harness lives in the `duckchat` crate. This module wraps it
//! for iced: each live chat session gets a `Subscription` that spawns a
//! `duckchat` worker, forwards provider events, and emits duckboard-specific
//! `Ready` / `CommandsAvailable` / `ProcessExited` bookends.

use std::path::PathBuf;

use iced::Subscription;
use tokio::sync::mpsc;

pub use duckchat::{AgentHandle, SlashCommand};

use duckchat::claude_code::ClaudeCodeProvider;

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
        model: Option<String>,
        input_tokens: usize,
        output_tokens: usize,
        context_window: Option<usize>,
    },
    SessionIdUpdated {
        session_id: String,
    },
    TurnComplete,
    Error(String),
    ProcessExited,
}

// ── Subscription ────────────────────────────────────────────────────────────

/// Create a subscription that manages one agent chat session.
///
/// `key` is an opaque routing token that gets echoed back with every event so
/// the caller can demultiplex when several sessions run in parallel. Matches
/// the shape of the previous hand-rolled implementation.
pub fn agent_subscription(
    key: String,
    project_root: PathBuf,
) -> Subscription<(String, AgentEvent)> {
    Subscription::run_with((key, project_root.clone()), |(key, root)| {
        use iced::futures::StreamExt;
        let key = key.clone();
        agent_stream(root.clone()).map(move |e| (key.clone(), e))
    })
}

fn agent_stream(project_root: PathBuf) -> impl iced::futures::Stream<Item = AgentEvent> {
    iced::stream::channel(
        256,
        |mut sender: iced::futures::channel::mpsc::Sender<AgentEvent>| async move {
            use duckchat::Provider;
            use iced::futures::SinkExt;

            let provider = ClaudeCodeProvider::new();
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
                    duckchat::AgentEvent::ContentDelta { text } => {
                        AgentEvent::ContentDelta { text }
                    }
                    duckchat::AgentEvent::ReasoningDelta { .. } => continue,
                    duckchat::AgentEvent::ToolUse { id, name, input } => {
                        AgentEvent::ToolUse { id, name, input }
                    }
                    duckchat::AgentEvent::ToolResult { id, name, output } => {
                        AgentEvent::ToolResult { id, name, output }
                    }
                    duckchat::AgentEvent::ModelUpdate { model } => AgentEvent::UsageUpdate {
                        model: Some(model),
                        input_tokens: 0,
                        output_tokens: 0,
                        context_window: None,
                    },
                    duckchat::AgentEvent::UsageUpdate(usage) => AgentEvent::UsageUpdate {
                        model: None,
                        input_tokens: usage.input_tokens.unwrap_or(0),
                        output_tokens: usage.output_tokens.unwrap_or(0),
                        context_window: usage.context_window,
                    },
                    duckchat::AgentEvent::SessionIdUpdated { session_id } => {
                        AgentEvent::SessionIdUpdated { session_id }
                    }
                    duckchat::AgentEvent::TurnComplete => AgentEvent::TurnComplete,
                    duckchat::AgentEvent::Error(msg) => AgentEvent::Error(msg),
                };
                if sender.send(mapped).await.is_err() {
                    break;
                }
            }

            let _ = sender.send(AgentEvent::ProcessExited).await;
        },
    )
}
