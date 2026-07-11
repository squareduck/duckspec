//! Agent process launch factory.
//!
//! The launch owns the **final** argv (login-shell wrap and harness flags
//! already applied). The ACP client spawns it as-is and never appends
//! harness-specific arguments.

use std::sync::Arc;

use tokio::process::Command;

/// Builds the agent child command for a harness.
///
/// Providers construct this once (or per working directory if needed). The
/// client calls [`AgentLaunch::command`] at spawn time and does not mutate the
/// program or args beyond stdio wiring and `current_dir`.
#[derive(Clone)]
pub struct AgentLaunch {
    /// Final argv already wrapped (login shell if needed). Client does not
    /// append harness-specific flags.
    pub build: Arc<dyn Fn() -> Command + Send + Sync>,
}

impl AgentLaunch {
    /// Construct a launch from a spawn factory.
    pub fn new(build: impl Fn() -> Command + Send + Sync + 'static) -> Self {
        Self {
            build: Arc::new(build),
        }
    }

    /// Build a fresh [`Command`] for one spawn.
    pub fn command(&self) -> Command {
        (self.build)()
    }
}
