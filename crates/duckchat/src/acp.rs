//! Shared Agent Client Protocol (ACP) client.
//!
//! Providers supply an [`AgentLaunch`]; this module owns spawn, session
//! open/resume, profile event mapping, and main/oneshot process heat.

pub mod ask_user;
mod event;
mod launch;
mod runtime;
mod turn;

/// Grok ask-user encode/decode (re-export under the name used by harness tests).
pub use ask_user as turn_ask_user;
pub use event::map_update;
pub use launch::AgentLaunch;
#[cfg(test)]
pub(crate) use runtime::pick_oneshot_model;
pub use runtime::{AcpMainRuntime, AcpOneshotRuntime};
pub use turn::{AcpModel, AcpTurn, InitResult, PromptResult};
