//! Shared Agent Client Protocol (ACP) client.
//!
//! Providers supply an [`AgentLaunch`]; this module owns spawn, session
//! open/resume, profile event mapping, and main/oneshot process heat.

mod event;
mod launch;
mod runtime;
mod turn;

pub use event::map_update;
pub use launch::AgentLaunch;
pub use runtime::{AcpMainRuntime, AcpOneshotRuntime};
pub use turn::{AcpModel, AcpTurn, InitResult, PromptResult};
