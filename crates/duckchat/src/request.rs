use std::collections::HashMap;
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
    /// Caller-supplied attachments keyed by id. The `prompt` text may embed
    /// `[label](attach:<id>)` markdown links referring to entries here; the
    /// provider walks the text and interleaves the attachment payloads into
    /// the wire-format message.
    pub attachments: HashMap<String, Attachment>,
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
            attachments: HashMap::new(),
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

#[derive(Debug, Clone)]
pub struct Attachment {
    /// Human-readable label (e.g. clipboard timestamp filename).
    pub label: String,
    /// IANA media type, e.g. `image/png`.
    pub media_type: String,
    /// Raw payload bytes.
    pub bytes: Vec<u8>,
}

/// Result of a completed turn.
#[derive(Debug, Clone)]
pub struct TurnOutcome {
    pub session_id: String,
}

/// Input to `Provider::title_summary`. Carries the user's opening message
/// plus an arbitrary list of caller-supplied context hints. Each hint is
/// rendered as a `Hint: ...` line in the summariser prompt, giving the
/// caller a place to inject domain-specific nudges (e.g. "user is
/// implementing step 03-foo") or scope orientation without duckchat needing
/// to know about them.
///
/// Deliberately excludes the assistant's reply: the title should reflect
/// the user's intent for the session, not whatever the agent said in
/// response. Slash commands carry intent in the hint; the user message
/// itself disambiguates everything else.
#[derive(Debug, Clone)]
pub struct TitleRequest {
    pub user_message: String,
    pub context_hints: Vec<String>,
}

impl TitleRequest {
    pub fn new(user_message: impl Into<String>) -> Self {
        Self {
            user_message: user_message.into(),
            context_hints: Vec::new(),
        }
    }
}
