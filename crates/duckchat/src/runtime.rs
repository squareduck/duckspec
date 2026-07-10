//! Harness-agnostic process runtimes for main turns and oneshot work.
//!
//! A [`Provider`](crate::provider::Provider) is discovery + factory; the worker
//! owns a [`MainRuntime`] and an [`OneshotRuntime`] for the chat's lifetime.
//! Harnesses that cannot keep a process warm implement no-op heat (spawn per
//! call).

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::cancel::CancelToken;
use crate::error::Error;
use crate::event::AgentEvent;
use crate::request::{TurnOutcome, TurnRequest};

/// Which cheap-model framing the oneshot path is serving.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OneshotKind {
    Title,
    ReplySuggest,
}

/// Long-lived main-turn process (or cold equivalent). One per chat worker.
#[async_trait]
pub trait MainRuntime: Send {
    /// Spawn + handshake if cold. Idempotent when already hot.
    async fn ensure_hot(&mut self) -> Result<(), Error>;

    /// Run one turn. Caller supplies resume id via `req.session_id`.
    /// Streams into `events`. On cancel, kill the process and leave cold.
    async fn run_turn(
        &mut self,
        req: TurnRequest,
        events: mpsc::Sender<AgentEvent>,
        cancel: CancelToken,
    ) -> Result<TurnOutcome, Error>;

    /// Drop any held child. Safe if already cold.
    async fn shutdown(&mut self);
}

/// Long-lived cheap-model process for title + reply suggestions (or cold
/// equivalent).
#[async_trait]
pub trait OneshotRuntime: Send {
    /// Spawn + handshake if cold. Idempotent when already hot.
    async fn ensure_hot(&mut self) -> Result<(), Error>;

    /// Single isolated prompt; returns raw assistant text (caller parses).
    /// Must not use tools. Does not resume a prior oneshot conversation.
    async fn prompt(&mut self, model_hint: OneshotKind, text: String) -> Result<String, Error>;

    /// Open a fresh logical session (or equivalent isolation) while keeping
    /// the process hot when possible. Called after each successful prompt (N=1).
    async fn rotate(&mut self) -> Result<(), Error>;

    /// Drop any held child. Safe if already cold.
    async fn shutdown(&mut self);
}
