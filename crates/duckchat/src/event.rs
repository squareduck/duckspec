use serde::{Deserialize, Serialize};

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
    /// The agent finished its turn successfully.
    TurnComplete,
    /// An error occurred during the turn.
    Error(String),
}

/// Token-usage / context-window snapshot. All fields are optional — a single
/// event may carry just the input/output delta, or just the context window.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: Option<usize>,
    pub output_tokens: Option<usize>,
    pub context_window: Option<usize>,
}
