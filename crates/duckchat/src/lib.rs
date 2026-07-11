//! Agent harness for duckboard / duckspec.
//!
//! Wraps an LLM-backed agent (today: the `claude` CLI) behind a provider trait
//! so callers can drive prompt turns without caring which backend runs them.
//! Events (content deltas, tool use/results, usage updates) stream back through
//! a plain `tokio::sync::mpsc` receiver; GUI integration (e.g. wrapping into an
//! iced `Subscription`) is the caller's job.

mod attach;
pub mod cancel;
pub mod cwd;
pub mod error;
pub mod event;
pub mod hook;
pub mod provider;
pub mod reply_suggest;
pub mod request;
pub mod runtime;
pub mod title;
pub mod worker;

pub mod claude_code;
pub mod grok;

pub use cancel::CancelToken;
pub use cwd::normalize_cwd;
pub use error::Error;
pub use event::{AgentEvent, Usage};
pub use hook::{ContextHook, HookOutput};
pub use provider::{
    Capabilities, ModelInfo, ModelRef, Provider, SlashCommand, SlashCommandKind,
};
pub use reply_suggest::parse_replies;
pub use request::{
    Attachment, ReasoningMode, ReplySuggestionRequest, TitleRequest, ToolPolicy, TurnOutcome,
    TurnRequest,
};
pub use runtime::{MainRuntime, OneshotKind, OneshotRuntime};
pub use title::{build_title_prompt, clean_title};
pub use worker::{AgentCommand, AgentHandle, ONESHOT_CALL_BUDGET, spawn_worker};
