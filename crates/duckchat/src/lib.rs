//! Agent harness for duckboard / duckspec.
//!
//! Wraps an LLM-backed agent (today: the `claude` CLI) behind a provider trait
//! so callers can drive prompt turns without caring which backend runs them.
//! Events (content deltas, tool use/results, usage updates) stream back through
//! a plain `tokio::sync::mpsc` receiver; GUI integration (e.g. wrapping into an
//! iced `Subscription`) is the caller's job.

pub mod cancel;
pub mod error;
pub mod event;
pub mod hook;
pub mod provider;
pub mod request;
pub mod worker;

pub mod claude_code;

pub use cancel::CancelToken;
pub use error::Error;
pub use event::{AgentEvent, Usage};
pub use hook::{ContextHook, HookOutput};
pub use provider::{Capabilities, ModelInfo, Provider, SlashCommand};
pub use request::{Attachment, ReasoningMode, TitleRequest, ToolPolicy, TurnOutcome, TurnRequest};
pub use worker::{AgentCommand, AgentHandle, spawn_worker};
