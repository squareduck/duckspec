//! Official `codex app-server` spawn + JSON-RPC client.
//!
//! Host-facing ACP never speaks App Server; only this module does.

mod app_server;
pub(crate) mod ask_user;
mod content;
mod map;
mod spawn;

pub use app_server::{AppServer, AppServerError, TurnStreamEvent};
pub use content::acp_prompt_to_turn_input;
pub use map::map_notification;
pub use spawn::{CodexSpawnFactory, default_spawn_factory};

#[cfg(test)]
pub use spawn::counting_factory;
