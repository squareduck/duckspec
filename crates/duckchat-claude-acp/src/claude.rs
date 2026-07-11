//! Official `claude` CLI child: spawn, duplex heat, protocol, and profile map.

pub(crate) mod ask_user;
mod content;
pub(crate) mod duplex;
mod map;
mod protocol;
mod spawn;

pub(crate) use content::acp_prompt_to_claude_content;
pub(crate) use duplex::{ClaudeDuplex, ClaudeSpawnFactory, default_spawn_factory};
#[cfg(test)]
pub(crate) use duplex::counting_factory;
