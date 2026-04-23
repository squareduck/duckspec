use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Everything a provider needs to run a single prompt turn.
#[derive(Debug, Clone)]
pub struct TurnRequest {
    /// The user's prompt text, already assembled by the caller. Any
    /// duckspec-specific context (spec content, diff, etc.) must be folded in
    /// before the request reaches the provider.
    pub prompt: String,
    /// Extra system-prompt-style strings contributed by caller-side hooks.
    /// Providers prepend these to the prompt in order, separated by blank
    /// lines.
    pub system_additions: Vec<String>,
    /// Provider-specific session id to resume. `None` starts a fresh session.
    pub session_id: Option<String>,
    /// Working directory the provider should operate against (project root
    /// for Claude Code).
    pub working_dir: PathBuf,
    /// Optional model override. `None` lets the provider pick its default.
    pub model: Option<String>,
    /// Optional reasoning/thinking mode. Ignored by providers that don't
    /// surface this knob.
    pub reasoning: Option<ReasoningMode>,
    /// Tool-call permission policy.
    pub tools: ToolPolicy,
    /// Caller-supplied file or text attachments. Reserved; no current
    /// provider uses this yet.
    #[allow(dead_code)]
    pub attachments: Vec<Attachment>,
}

impl TurnRequest {
    pub fn new(prompt: impl Into<String>, working_dir: PathBuf) -> Self {
        Self {
            prompt: prompt.into(),
            system_additions: Vec::new(),
            session_id: None,
            working_dir,
            model: None,
            reasoning: None,
            tools: ToolPolicy::default(),
            attachments: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ReasoningMode {
    Off,
    Low,
    Medium,
    High,
}

/// How aggressively the provider should let the agent invoke tools.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum ToolPolicy {
    /// Skip every permission prompt (Claude Code's `bypassPermissions`).
    #[default]
    BypassAll,
    /// Default interactive behaviour — provider asks per tool.
    Interactive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub label: String,
    pub content: String,
}

/// Result of a completed turn.
#[derive(Debug, Clone)]
pub struct TurnOutcome {
    pub session_id: String,
}

/// Input to `Provider::title_summary`. Carries the opening user/assistant
/// exchange plus an arbitrary list of caller-supplied context hints. Each
/// hint is rendered as a `Hint: ...` line in the summariser prompt, giving
/// the caller a place to inject domain-specific nudges (e.g. "user is
/// implementing step 03-foo") without duckchat needing to know about them.
#[derive(Debug, Clone)]
pub struct TitleRequest {
    pub user_message: String,
    pub assistant_reply: String,
    pub context_hints: Vec<String>,
}

impl TitleRequest {
    pub fn new(user_message: impl Into<String>, assistant_reply: impl Into<String>) -> Self {
        Self {
            user_message: user_message.into(),
            assistant_reply: assistant_reply.into(),
            context_hints: Vec::new(),
        }
    }
}
