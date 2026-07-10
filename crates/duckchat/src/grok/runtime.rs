//! Grok process-hot main and oneshot runtimes.
//!
//! Each runtime holds an optional long-lived [`AcpTurn`] child. `ensure_hot`
//! spawns + `initialize` once; subsequent work reuses the process. Cancel kills
//! the main child and clears heat. Oneshot rotates to a fresh ACP session after
//! each successful prompt (N=1) while keeping the process when possible.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use futures::future::BoxFuture;
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::cancel::CancelToken;
use crate::error::Error;
use crate::event::AgentEvent;
use crate::request::{TurnOutcome, TurnRequest};
use crate::runtime::{MainRuntime, OneshotKind, OneshotRuntime};

use super::acp::{AcpModel, AcpTurn};
use super::assemble_content;
use super::event::map_update;
use super::pick_title_model;
use super::text_prompt_content;

/// Builds the base `Command` a turn spawns from.
pub(super) type Spawner = Arc<dyn Fn() -> Command + Send + Sync>;

/// Opens a fresh ACP transport (spawn + stdio). Production uses the real
/// spawner; tests inject a scripted peer factory so process reuse is observable
/// without a live `grok` binary.
type OpenChild = Arc<dyn Fn() -> BoxFuture<'static, Result<AcpTurn, Error>> + Send + Sync>;

fn open_from_spawner(spawn: Spawner, working_dir: PathBuf) -> OpenChild {
    Arc::new(move || {
        let cmd = spawn();
        let cwd = working_dir.clone();
        Box::pin(async move { AcpTurn::spawn_with(cmd, &cwd).await })
    })
}

/// Main path: one warm `grok agent stdio` process across turns.
pub struct GrokMainRuntime {
    open: OpenChild,
    working_dir: PathBuf,
    turn: Option<AcpTurn>,
    models: Vec<AcpModel>,
}

impl GrokMainRuntime {
    pub(super) fn new(spawn: Spawner, working_dir: &Path) -> Self {
        let working_dir = working_dir.to_path_buf();
        Self {
            open: open_from_spawner(spawn, working_dir.clone()),
            working_dir,
            turn: None,
            models: Vec::new(),
        }
    }

    /// Test seam: custom child opener (scripted peer) instead of a real spawn.
    #[cfg(test)]
    fn with_open(open: OpenChild, working_dir: PathBuf) -> Self {
        Self {
            open,
            working_dir,
            turn: None,
            models: Vec::new(),
        }
    }

    async fn drop_child(&mut self) {
        if let Some(mut turn) = self.turn.take() {
            turn.cancel().await;
        }
        self.models.clear();
    }

    fn clear_if_dead(&mut self) {
        if let Some(turn) = self.turn.as_mut()
            && !turn.process_alive()
        {
            self.turn = None;
            self.models.clear();
        }
    }
}

#[async_trait]
impl MainRuntime for GrokMainRuntime {
    async fn ensure_hot(&mut self) -> Result<(), Error> {
        self.clear_if_dead();
        if self.turn.is_some() {
            return Ok(());
        }
        let mut turn = (self.open)().await?;
        let init = turn.initialize().await?;
        self.models = init.models;
        self.turn = Some(turn);
        Ok(())
    }

    async fn run_turn(
        &mut self,
        req: TurnRequest,
        events: mpsc::Sender<AgentEvent>,
        cancel: CancelToken,
    ) -> Result<TurnOutcome, Error> {
        self.ensure_hot().await?;

        let cwd = if req.working_dir.as_os_str().is_empty() {
            self.working_dir.clone()
        } else {
            req.working_dir.clone()
        };

        let session_id = {
            let turn = self
                .turn
                .as_mut()
                .ok_or_else(|| Error::Other("main runtime hot but no turn".into()))?;
            match turn.open(req.session_id.as_deref(), &cwd).await {
                Ok(sid) => sid,
                Err(e) => {
                    // I/O / protocol failure leaves us cold for the next attempt.
                    self.drop_child().await;
                    return Err(e);
                }
            }
        };

        // Surface the session id before prompting so the caller can persist it
        // even if the turn is later interrupted. The worker re-emits it on a
        // successful outcome; both are idempotent for persistence.
        let _ = events
            .send(AgentEvent::SessionIdUpdated {
                session_id: session_id.clone(),
            })
            .await;

        let model = req.model.clone().unwrap_or_default();
        let context_window = self
            .models
            .iter()
            .find(|m| m.id == model)
            .and_then(|m| m.context_window);

        let content = assemble_content(&req);
        let result = {
            let turn = self
                .turn
                .as_mut()
                .ok_or_else(|| Error::Other("main runtime hot but no turn".into()))?;
            turn.prompt_events(
                &session_id,
                &content,
                &model,
                req.reasoning,
                context_window,
                &events,
                &cancel,
            )
            .await
        };

        match result {
            Ok(result) => match result.stop_reason.as_deref() {
                // `end_turn` (or an absent reason) is a clean completion; the
                // process stays hot for the next turn.
                Some("end_turn") | None => Ok(TurnOutcome { session_id }),
                Some(other) => {
                    self.drop_child().await;
                    Err(Error::Process(format!("grok stopped early: {other}")))
                }
            },
            Err(Error::Cancelled) => {
                // Cancel already killed the child inside the prompt loop.
                self.turn = None;
                self.models.clear();
                Err(Error::Cancelled)
            }
            Err(e) => {
                self.drop_child().await;
                Err(e)
            }
        }
    }

    async fn shutdown(&mut self) {
        self.drop_child().await;
    }
}

/// Oneshot path: warm process, fresh logical session every call (N=1).
pub struct GrokOneshotRuntime {
    open: OpenChild,
    working_dir: PathBuf,
    turn: Option<AcpTurn>,
    session_id: Option<String>,
    models: Vec<AcpModel>,
}

impl GrokOneshotRuntime {
    pub(super) fn new(spawn: Spawner, working_dir: &Path) -> Self {
        let working_dir = working_dir.to_path_buf();
        Self {
            open: open_from_spawner(spawn, working_dir.clone()),
            working_dir,
            turn: None,
            session_id: None,
            models: Vec::new(),
        }
    }

    #[cfg(test)]
    fn with_open(open: OpenChild, working_dir: PathBuf) -> Self {
        Self {
            open,
            working_dir,
            turn: None,
            session_id: None,
            models: Vec::new(),
        }
    }

    async fn drop_child(&mut self) {
        if let Some(mut turn) = self.turn.take() {
            turn.cancel().await;
        }
        self.session_id = None;
        self.models.clear();
    }

    fn clear_if_dead(&mut self) {
        if let Some(turn) = self.turn.as_mut()
            && !turn.process_alive()
        {
            self.turn = None;
            self.session_id = None;
            self.models.clear();
        }
    }

    async fn ensure_session(&mut self) -> Result<String, Error> {
        if let Some(sid) = self.session_id.clone() {
            return Ok(sid);
        }
        let turn = self
            .turn
            .as_mut()
            .ok_or_else(|| Error::Other("oneshot runtime has no process".into()))?;
        let sid = turn.open(None, &self.working_dir).await?;
        self.session_id = Some(sid.clone());
        Ok(sid)
    }
}

#[async_trait]
impl OneshotRuntime for GrokOneshotRuntime {
    async fn ensure_hot(&mut self) -> Result<(), Error> {
        self.clear_if_dead();
        if self.turn.is_none() {
            let mut turn = (self.open)().await?;
            let init = turn.initialize().await?;
            self.models = init.models;
            self.turn = Some(turn);
        }
        if self.session_id.is_none() {
            self.ensure_session().await?;
        }
        Ok(())
    }

    async fn prompt(&mut self, _model_hint: OneshotKind, text: String) -> Result<String, Error> {
        self.ensure_hot().await?;
        let session_id = self.ensure_session().await?;
        let model = pick_title_model(&self.models)
            .ok_or_else(|| Error::Other("grok advertised no models for oneshot".into()))?;
        let content = text_prompt_content(&text);

        // Cancel token is cooperative while prompt is polling; if the future is
        // abandoned (worker oneshot budget), the worker calls `shutdown` →
        // `drop_child` which kills ACP heat so the next `ensure_hot` can proceed.
        let cancel = CancelToken::new();
        let mut raw = String::new();
        let result = {
            let turn = self
                .turn
                .as_mut()
                .ok_or_else(|| Error::Other("oneshot runtime hot but no turn".into()))?;
            turn.prompt(
                &session_id,
                &content,
                &model,
                None,
                &mut |params| {
                    if let Some(AgentEvent::ContentDelta { text }) = map_update(params, None) {
                        raw.push_str(&text);
                    }
                },
                &cancel,
            )
            .await
        };

        match result {
            Ok(_) => Ok(raw),
            Err(e) => {
                self.drop_child().await;
                Err(e)
            }
        }
    }

    async fn rotate(&mut self) -> Result<(), Error> {
        // Prefer session/new on the live child; fall back to respawn.
        if let Some(turn) = self.turn.as_mut() {
            match turn.open(None, &self.working_dir).await {
                Ok(sid) => {
                    self.session_id = Some(sid);
                    return Ok(());
                }
                Err(_) => {
                    // Drop and re-hot below.
                }
            }
        }
        self.drop_child().await;
        self.ensure_hot().await
    }

    async fn shutdown(&mut self) {
        self.drop_child().await;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use serde_json::{Value, json};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, duplex};
    use tokio::sync::mpsc;

    use super::*;
    use crate::request::TurnRequest;

    /// Spawn-counting factory that opens a duplex fake ACP peer.
    fn counting_open(spawn_count: Arc<AtomicUsize>) -> OpenChild {
        Arc::new(move || {
            spawn_count.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(fake_acp_peer().await) })
        })
    }

    /// Interactive fake agent: answers initialize / session/new|load / prompt.
    async fn fake_acp_peer() -> AcpTurn {
        let (client, server) = duplex(16 * 1024);
        let (server_read, mut server_write) = tokio::io::split(server);
        let (client_read, client_write) = tokio::io::split(client);

        tokio::spawn(async move {
            let mut reader = BufReader::new(server_read);
            let mut line = String::new();
            let mut next_sess = 1u64;
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                let msg: Value = match serde_json::from_str(line.trim()) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
                let reply = match method {
                    "initialize" => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "protocolVersion": 1,
                            "agentCapabilities": { "loadSession": true },
                            "_meta": {
                                "modelState": {
                                    "availableModels": [{
                                        "modelId": "grok-composer-2.5-fast",
                                        "name": "Composer Fast",
                                        "_meta": { "totalContextTokens": 128000 }
                                    }]
                                }
                            }
                        }
                    }),
                    "session/new" => {
                        let sid = format!("oneshot-sess-{next_sess}");
                        next_sess += 1;
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": { "sessionId": sid }
                        })
                    }
                    "session/load" => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {}
                    }),
                    "session/prompt" => {
                        // Stream a short content delta, then end the turn.
                        let update = json!({
                            "jsonrpc": "2.0",
                            "method": "session/update",
                            "params": {
                                "update": {
                                    "sessionUpdate": "agent_message_chunk",
                                    "content": { "type": "text", "text": "ok" }
                                }
                            }
                        });
                        let _ = write_line(&mut server_write, &update).await;
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": { "stopReason": "end_turn" }
                        })
                    }
                    _ => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {}
                    }),
                };
                if write_line(&mut server_write, &reply).await.is_err() {
                    break;
                }
            }
        });

        AcpTurn::from_transport(Box::pin(client_write), Box::pin(BufReader::new(client_read)))
    }

    /// Fake peer that hangs on the first `session/prompt` until the client
    /// disconnects (used for cancel tests).
    async fn fake_acp_peer_hang_on_prompt() -> AcpTurn {
        let (client, server) = duplex(16 * 1024);
        let (server_read, mut server_write) = tokio::io::split(server);
        let (client_read, client_write) = tokio::io::split(client);

        tokio::spawn(async move {
            let mut reader = BufReader::new(server_read);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                let msg: Value = match serde_json::from_str(line.trim()) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
                match method {
                    "initialize" => {
                        let reply = json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "protocolVersion": 1,
                                "agentCapabilities": { "loadSession": true },
                                "_meta": {
                                    "modelState": {
                                        "availableModels": [{
                                            "modelId": "grok-composer-2.5-fast",
                                            "name": "Composer Fast",
                                            "_meta": { "totalContextTokens": 128000 }
                                        }]
                                    }
                                }
                            }
                        });
                        if write_line(&mut server_write, &reply).await.is_err() {
                            break;
                        }
                    }
                    "session/new" | "session/load" => {
                        let reply = if method == "session/new" {
                            json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": { "sessionId": "hang-sess" }
                            })
                        } else {
                            json!({ "jsonrpc": "2.0", "id": id, "result": {} })
                        };
                        if write_line(&mut server_write, &reply).await.is_err() {
                            break;
                        }
                    }
                    "session/prompt" => {
                        // Never respond — client must cancel while waiting.
                        let _ = tokio::time::sleep(Duration::from_secs(60)).await;
                    }
                    _ => {}
                }
            }
        });

        AcpTurn::from_transport(Box::pin(client_write), Box::pin(BufReader::new(client_read)))
    }

    async fn write_line(
        w: &mut (impl AsyncWriteExt + Unpin),
        msg: &Value,
    ) -> Result<(), std::io::Error> {
        let mut line = serde_json::to_string(msg).unwrap();
        line.push('\n');
        w.write_all(line.as_bytes()).await?;
        w.flush().await?;
        Ok(())
    }

    fn turn_req(session_id: Option<&str>) -> TurnRequest {
        let mut req = TurnRequest::new("hello", std::env::temp_dir());
        req.session_id = session_id.map(str::to_string);
        req.model = Some("grok-composer-2.5-fast".into());
        req
    }

    // @spec harness/grok Session lifecycle and resume: A second turn on a hot path reuses the process
    #[tokio::test]
    async fn a_second_turn_on_a_hot_path_reuses_the_process() {
        let spawns = Arc::new(AtomicUsize::new(0));
        let mut rt = GrokMainRuntime::with_open(counting_open(spawns.clone()), std::env::temp_dir());
        let (tx, mut rx) = mpsc::channel(32);

        rt.ensure_hot().await.unwrap();
        let out1 = rt
            .run_turn(turn_req(None), tx.clone(), CancelToken::new())
            .await
            .unwrap();
        assert!(!out1.session_id.is_empty());

        let out2 = rt
            .run_turn(
                turn_req(Some(&out1.session_id)),
                tx.clone(),
                CancelToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(out2.session_id, out1.session_id);

        assert_eq!(
            spawns.load(Ordering::SeqCst),
            1,
            "second turn must reuse the process (single spawn)"
        );

        // Drain events so the channel is not full for anything else.
        while rx.try_recv().is_ok() {}
        rt.shutdown().await;
    }

    // @spec harness/grok Session lifecycle and resume: After cancel, the next turn can spawn and resume
    #[tokio::test]
    async fn after_cancel_the_next_turn_can_spawn_and_resume() {
        let spawns = Arc::new(AtomicUsize::new(0));
        let hang_open: OpenChild = {
            let spawns = spawns.clone();
            let first = Arc::new(std::sync::atomic::AtomicBool::new(true));
            Arc::new(move || {
                spawns.fetch_add(1, Ordering::SeqCst);
                let is_first = first.swap(false, Ordering::SeqCst);
                Box::pin(async move {
                    if is_first {
                        Ok(fake_acp_peer_hang_on_prompt().await)
                    } else {
                        Ok(fake_acp_peer().await)
                    }
                })
            })
        };
        let rt = GrokMainRuntime::with_open(hang_open, std::env::temp_dir());
        let (tx, mut rx) = mpsc::channel(32);

        let cancel = CancelToken::new();
        let cancel2 = cancel.clone();
        let mut rt_holder = Some(rt);
        let turn = tokio::spawn(async move {
            let mut rt = rt_holder.take().unwrap();
            let result = rt
                .run_turn(turn_req(Some("prior-sess-id")), tx, cancel2)
                .await;
            (rt, result)
        });

        // Let the hang-on-prompt peer reach session/prompt, then cancel.
        tokio::time::sleep(Duration::from_millis(80)).await;
        cancel.cancel();

        let (mut rt, result) = turn.await.unwrap();
        assert!(matches!(result, Err(Error::Cancelled)));

        // Later turn may spawn again and resume the prior conversation id.
        let (tx2, mut rx2) = mpsc::channel(32);
        let out = rt
            .run_turn(
                turn_req(Some("prior-sess-id")),
                tx2,
                CancelToken::new(),
            )
            .await
            .expect("turn after cancel should complete");
        assert_eq!(out.session_id, "prior-sess-id");
        assert!(
            spawns.load(Ordering::SeqCst) >= 2,
            "cancel kills heat; next turn spawns again"
        );

        while rx.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        rt.shutdown().await;
    }

    // @spec harness/grok Warm oneshot path: A second oneshot call does not resume the prior oneshot session
    #[tokio::test]
    async fn a_second_oneshot_call_does_not_resume_the_prior_oneshot_session() {
        let spawns = Arc::new(AtomicUsize::new(0));
        let mut rt =
            GrokOneshotRuntime::with_open(counting_open(spawns.clone()), std::env::temp_dir());

        rt.ensure_hot().await.unwrap();
        let sid1 = rt.session_id.clone().expect("session after ensure_hot");
        let _ = rt
            .prompt(OneshotKind::Title, "first oneshot prompt".into())
            .await
            .unwrap();
        // Worker calls rotate after success; do the same.
        rt.rotate().await.unwrap();
        let sid_after_rotate = rt.session_id.clone().expect("session after rotate");
        assert_ne!(
            sid1, sid_after_rotate,
            "rotate must open a fresh oneshot session"
        );

        let _ = rt
            .prompt(OneshotKind::Title, "second oneshot prompt".into())
            .await
            .unwrap();
        // Second prompt uses the post-rotate session, not the first.
        assert_eq!(rt.session_id.as_deref(), Some(sid_after_rotate.as_str()));
        assert_ne!(rt.session_id.as_deref(), Some(sid1.as_str()));

        rt.shutdown().await;
    }

    // @spec harness/grok Warm oneshot path: An oneshot call on a hot path reuses the process
    #[tokio::test]
    async fn an_oneshot_call_on_a_hot_path_reuses_the_process() {
        let spawns = Arc::new(AtomicUsize::new(0));
        let mut rt =
            GrokOneshotRuntime::with_open(counting_open(spawns.clone()), std::env::temp_dir());

        rt.ensure_hot().await.unwrap();
        assert_eq!(spawns.load(Ordering::SeqCst), 1);

        let _ = rt
            .prompt(OneshotKind::Title, "title while hot".into())
            .await
            .unwrap();
        rt.rotate().await.unwrap();
        let _ = rt
            .prompt(OneshotKind::ReplySuggest, "reply while hot".into())
            .await
            .unwrap();

        assert_eq!(
            spawns.load(Ordering::SeqCst),
            1,
            "oneshot calls on a hot path must not spawn a new process"
        );
        rt.shutdown().await;
    }

    /// Abandoned/hung oneshot + `shutdown` cold-resets heat so a later oneshot works.
    #[tokio::test]
    async fn abandoned_oneshot_shutdown_allows_later_oneshot() {
        let spawns = Arc::new(AtomicUsize::new(0));
        let hang_then_ok: OpenChild = {
            let spawns = spawns.clone();
            let first = Arc::new(std::sync::atomic::AtomicBool::new(true));
            Arc::new(move || {
                spawns.fetch_add(1, Ordering::SeqCst);
                let is_first = first.swap(false, Ordering::SeqCst);
                Box::pin(async move {
                    if is_first {
                        Ok(fake_acp_peer_hang_on_prompt().await)
                    } else {
                        Ok(fake_acp_peer().await)
                    }
                })
            })
        };
        let mut rt = GrokOneshotRuntime::with_open(hang_then_ok, std::env::temp_dir());

        rt.ensure_hot().await.unwrap();
        // Hang on session/prompt until the budget aborts this future (worker does
        // the same with `timeout` then `shutdown`).
        let hung = tokio::time::timeout(
            Duration::from_millis(80),
            rt.prompt(OneshotKind::Title, "will hang".into()),
        )
        .await;
        assert!(hung.is_err(), "first oneshot should not finish within budget");

        // Worker cold-reset path after Timeout.
        rt.shutdown().await;

        // Later oneshot on the same runtime can complete (re-hot).
        rt.ensure_hot().await.unwrap();
        let text = rt
            .prompt(OneshotKind::Title, "after recover".into())
            .await
            .expect("oneshot after hang+shutdown");
        assert!(!text.is_empty());
        assert!(
            spawns.load(Ordering::SeqCst) >= 2,
            "shutdown drops heat; next ensure_hot may spawn again"
        );
        rt.shutdown().await;
    }
}
