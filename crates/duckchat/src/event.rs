use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

/// Events streamed from a provider back to the caller during a prompt turn.
///
/// Shape is intentionally provider-neutral: the Claude Code provider is the
/// only current implementation, but future providers (native Anthropic/OpenAI,
/// opencode, etc.) should be able to emit the same event stream.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// The provider updated the active model name for this turn.
    ModelUpdate { model: String },
    /// Streaming text chunk from the agent's response.
    ContentDelta { text: String },
    /// Streaming reasoning/thinking chunk (separate channel from content).
    #[allow(dead_code)]
    ReasoningDelta { text: String },
    /// Agent started a tool call.
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    /// A previously-announced tool call completed.
    ToolResult {
        id: String,
        name: String,
        output: String,
    },
    /// Token usage / context-window telemetry update.
    UsageUpdate(Usage),
    /// The provider assigned or rotated a session id for this conversation.
    /// Callers should persist this to enable multi-turn resume.
    SessionIdUpdated { session_id: String },
    /// A stored resume id could not be loaded (session file missing / cwd key
    /// mismatch). The worker has already forgotten the id; the UI should drop
    /// its persisted copy and re-dispatch the turn as a fresh session with a
    /// history preamble.
    SessionNotFound,
    /// Mid-turn structured choice. Host must
    /// [`crate::worker::AgentHandle::answer_user_choice`] or turn cancel ends it.
    UserChoiceRequest(UserChoiceRequest),
    /// The agent finished its turn successfully.
    TurnComplete,
    /// An error occurred during the turn.
    Error(String),
}

/// One option in a mid-turn structured choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserChoiceOption {
    /// Wire option id / answer key.
    pub id: String,
    /// Chip label shown to the host.
    pub label: String,
}

/// Host-facing mid-turn structured choice request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserChoiceRequest {
    /// Host-local correlation id; maps to a pending JSON-RPC request inside the worker.
    pub correlation_id: u64,
    /// Question text when known.
    pub prompt: Option<String>,
    /// Options to present (typically 1..=9).
    pub options: Vec<UserChoiceOption>,
    /// When true, the host may cancel (⌘⌫).
    pub allow_cancel: bool,
}

/// Host answer to a pending [`UserChoiceRequest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserChoiceAnswer {
    /// Chip / structured option pick.
    Selected { option_id: String },
    /// Composer freeform while awaiting — custom answer text (not cancel).
    Custom { text: String },
    /// Esc / pure dismiss — harness skip or deny.
    Cancelled,
}

/// Shared map of parked mid-turn choices. The worker and ACP main path share one
/// instance so the host can answer while `run_turn` is blocked.
#[derive(Debug, Default)]
pub struct PendingUserChoices {
    next_id: AtomicU64,
    waiters: Mutex<HashMap<u64, oneshot::Sender<UserChoiceAnswer>>>,
}

impl PendingUserChoices {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Register a waiter and return `(correlation_id, receiver)`.
    pub fn park(&self) -> (u64, oneshot::Receiver<UserChoiceAnswer>) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let (tx, rx) = oneshot::channel();
        self.waiters
            .lock()
            .expect("pending choices lock")
            .insert(id, tx);
        (id, rx)
    }

    /// Deliver a host answer. No-op if the id is unknown or already completed.
    pub fn answer(&self, correlation_id: u64, answer: UserChoiceAnswer) {
        if let Some(tx) = self
            .waiters
            .lock()
            .expect("pending choices lock")
            .remove(&correlation_id)
        {
            let _ = tx.send(answer);
        }
    }

    /// Complete every waiter as cancelled (turn cancel / shutdown).
    pub fn cancel_all(&self) {
        let waiters: Vec<_> = self
            .waiters
            .lock()
            .expect("pending choices lock")
            .drain()
            .map(|(_, tx)| tx)
            .collect();
        for tx in waiters {
            let _ = tx.send(UserChoiceAnswer::Cancelled);
        }
    }

    /// Drop a waiter without answering (e.g. after local cancel already wrote).
    pub fn forget(&self, correlation_id: u64) {
        self.waiters
            .lock()
            .expect("pending choices lock")
            .remove(&correlation_id);
    }
}

/// Token-usage / context-window snapshot. All fields are optional — a single
/// event may carry just the input/output delta, or just the context window.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: Option<usize>,
    pub output_tokens: Option<usize>,
    pub context_window: Option<usize>,
}
