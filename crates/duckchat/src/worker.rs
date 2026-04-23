//! Long-lived command loop that owns a `Provider` and serialises prompt turns.
//!
//! The worker takes ownership of one provider and one event sender, and exposes
//! an [`AgentHandle`] to the caller. The handle is cheap to clone and can be
//! stored in UI state. Each `send_prompt` enqueues a turn; the worker runs
//! them in order.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::cancel::CancelToken;
use crate::event::AgentEvent;
use crate::provider::Provider;
use crate::request::TurnRequest;

/// Commands the caller can queue on the worker.
#[derive(Debug)]
pub enum AgentCommand {
    /// Run a prompt turn. Convenience helpers on [`AgentHandle`] construct
    /// this.
    RunTurn(TurnRequest),
    /// Seed the session id used by the next turn. Useful when resuming a
    /// previously-persisted conversation — the caller knows the id before the
    /// worker has seen a turn.
    SetSessionId(String),
    /// Stop processing further commands and return.
    Shutdown,
}

/// Cloneable handle for driving a worker.
#[derive(Clone)]
pub struct AgentHandle {
    cancel: CancelToken,
    tx: mpsc::UnboundedSender<AgentCommand>,
    working_dir: PathBuf,
}

impl AgentHandle {
    /// Queue a `TurnRequest` directly.
    pub fn send_turn(&self, req: TurnRequest) {
        let _ = self.tx.send(AgentCommand::RunTurn(req));
    }

    /// Convenience: build a minimal `TurnRequest` from `prompt` using the
    /// handle's working dir and queue it.
    pub fn send_prompt(&self, prompt: String) {
        self.send_turn(TurnRequest::new(prompt, self.working_dir.clone()));
    }

    pub fn set_session_id(&self, session_id: String) {
        let _ = self.tx.send(AgentCommand::SetSessionId(session_id));
    }

    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    pub fn shutdown(&self) {
        self.cancel.cancel();
        let _ = self.tx.send(AgentCommand::Shutdown);
    }

    pub fn working_dir(&self) -> &std::path::Path {
        &self.working_dir
    }
}

impl std::fmt::Debug for AgentHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentHandle")
            .field("working_dir", &self.working_dir)
            .finish()
    }
}

/// Spawn a worker task that drives `provider` and forwards events into
/// `events`. The caller holds the returned `AgentHandle`; the worker exits
/// when the handle is dropped (command channel closes) or `shutdown()` is
/// called.
///
/// The worker picks up a persisted session id the first time
/// `AgentCommand::SetSessionId` arrives, and updates it after every
/// successful turn. It reuses the id across turns unless the provider rotates
/// it.
pub fn spawn_worker<P: Provider + 'static>(
    provider: P,
    working_dir: PathBuf,
    events: mpsc::Sender<AgentEvent>,
) -> AgentHandle {
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<AgentCommand>();
    let cancel = CancelToken::new();
    let handle = AgentHandle {
        cancel: cancel.clone(),
        tx: cmd_tx,
        working_dir: working_dir.clone(),
    };

    let provider = Arc::new(provider);

    tokio::spawn(async move {
        let mut session_id: Option<String> = None;

        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                AgentCommand::RunTurn(mut req) => {
                    cancel.reset();
                    if req.session_id.is_none() {
                        req.session_id = session_id.clone();
                    }
                    let outcome = provider
                        .run_turn(req, events.clone(), cancel.clone())
                        .await;
                    let send_result = match outcome {
                        Ok(out) => {
                            let changed = session_id.as_deref() != Some(out.session_id.as_str());
                            session_id = Some(out.session_id.clone());
                            let mut r = Ok(());
                            if changed {
                                r = events
                                    .send(AgentEvent::SessionIdUpdated {
                                        session_id: out.session_id,
                                    })
                                    .await
                                    .map_err(|_| ());
                            }
                            if r.is_ok() {
                                r = events
                                    .send(AgentEvent::TurnComplete)
                                    .await
                                    .map_err(|_| ());
                            }
                            r
                        }
                        Err(crate::error::Error::Cancelled) => events
                            .send(AgentEvent::TurnComplete)
                            .await
                            .map_err(|_| ()),
                        Err(e) => events
                            .send(AgentEvent::Error(e.to_string()))
                            .await
                            .map_err(|_| ()),
                    };
                    // If the receiver is gone (subscription torn down) there's
                    // no point in continuing to process queued commands.
                    if send_result.is_err() {
                        break;
                    }
                }
                AgentCommand::SetSessionId(sid) => {
                    session_id = Some(sid);
                }
                AgentCommand::Shutdown => break,
            }
        }
    });

    handle
}
