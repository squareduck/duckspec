//! Line-delimited JSON-RPC client over a `codex app-server` child stdio.
//!
//! Wire format omits `"jsonrpc":"2.0"` (Codex convention). The reader demuxes
//! responses, auto-allows ordinary approval requests, parks structured
//! user-input for the agent, and surfaces notifications.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, mpsc, oneshot};
use tracing::warn;

use super::ask_user::{
    UserInputDecision, auto_allow_approval_result, encode_user_input_rpc,
    is_ordinary_approval_method, is_user_input_method,
};
use super::spawn::CodexSpawnFactory;

/// Errors from the App Server process or protocol.
#[derive(Debug)]
pub enum AppServerError {
    Spawn(String),
    Process(String),
    Protocol(String),
    SessionNotFound(String),
}

impl std::fmt::Display for AppServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppServerError::Spawn(m)
            | AppServerError::Process(m)
            | AppServerError::Protocol(m)
            | AppServerError::SessionNotFound(m) => write!(f, "{m}"),
        }
    }
}

impl AppServerError {
    pub fn is_session_not_found(&self) -> bool {
        matches!(self, AppServerError::SessionNotFound(_))
    }
}

/// Result of a completed turn against App Server.
#[derive(Debug)]
#[allow(dead_code)]
pub struct TurnOutcome {
    pub turn_id: String,
    pub turn: Value,
}

/// Parked `item/tool/requestUserInput` awaiting a host decision.
#[derive(Debug)]
pub struct UserInputNeed {
    pub rpc_id: Value,
    pub params: Value,
}

/// Events the agent drains while a turn is in flight.
#[derive(Debug)]
pub enum TurnStreamEvent {
    Notification(Value),
    UserInput(UserInputNeed),
}

type PendingMap = HashMap<u64, oneshot::Sender<Result<Value, AppServerError>>>;

/// One process-hot `codex app-server` child.
pub struct AppServer {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    next_id: AtomicU64,
    pending: Arc<Mutex<PendingMap>>,
    #[allow(dead_code)]
    notif_tx: mpsc::UnboundedSender<Value>,
    notif_rx: Arc<Mutex<mpsc::UnboundedReceiver<Value>>>,
    #[allow(dead_code)]
    user_input_tx: mpsc::UnboundedSender<UserInputNeed>,
    user_input_rx: Arc<Mutex<mpsc::UnboundedReceiver<UserInputNeed>>>,
    reader: tokio::task::JoinHandle<()>,
}

impl AppServer {
    /// Spawn via `factory`, run initialize + initialized handshake.
    pub async fn connect(factory: &CodexSpawnFactory) -> Result<Self, AppServerError> {
        let mut cmd: Command = factory();
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| AppServerError::Spawn(format!("spawn codex app-server: {e}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppServerError::Spawn("child stdin missing".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppServerError::Spawn("child stdout missing".into()))?;

        let stdin = Arc::new(Mutex::new(stdin));
        let pending: Arc<Mutex<PendingMap>> = Arc::new(Mutex::new(HashMap::new()));
        let (notif_tx, notif_rx) = mpsc::unbounded_channel();
        let notif_rx = Arc::new(Mutex::new(notif_rx));
        let (user_input_tx, user_input_rx) = mpsc::unbounded_channel();
        let user_input_rx = Arc::new(Mutex::new(user_input_rx));

        let reader = {
            let pending = Arc::clone(&pending);
            let stdin = Arc::clone(&stdin);
            let notif_tx = notif_tx.clone();
            let user_input_tx = user_input_tx.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout);
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => break,
                        Ok(_) => {}
                        Err(e) => {
                            warn!("app-server stdout read error: {e}");
                            break;
                        }
                    }
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let msg: Value = match serde_json::from_str(trimmed) {
                        Ok(v) => v,
                        Err(e) => {
                            warn!("app-server malformed line: {e}");
                            continue;
                        }
                    };
                    handle_incoming(msg, &pending, &stdin, &notif_tx, &user_input_tx).await;
                }
                let mut pend = pending.lock().await;
                for (_, tx) in pend.drain() {
                    let _ = tx.send(Err(AppServerError::Process(
                        "codex app-server exited".into(),
                    )));
                }
            })
        };

        let mut server = Self {
            child,
            stdin,
            next_id: AtomicU64::new(1),
            pending,
            notif_tx,
            notif_rx,
            user_input_tx,
            user_input_rx,
            reader,
        };

        server
            .request(
                "initialize",
                json!({
                    "clientInfo": {
                        "name": "duckchat-codex-acp",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                    "capabilities": {
                        "experimentalApi": true
                    }
                }),
            )
            .await?;
        server
            .notify("initialized", Value::Null)
            .await
            .map_err(|e| AppServerError::Protocol(e.to_string()))?;

        Ok(server)
    }

    pub fn alive(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(None) => true,
            Ok(Some(_)) | Err(_) => false,
        }
    }

    /// List models available to this Codex install (`model/list`).
    pub async fn model_list(&mut self) -> Result<Value, AppServerError> {
        self.request("model/list", json!({})).await
    }

    pub async fn thread_start(
        &mut self,
        cwd: &str,
        model: Option<&str>,
    ) -> Result<String, AppServerError> {
        let mut params = json!({
            "cwd": cwd,
            "approvalPolicy": "never",
        });
        if let Some(m) = model.filter(|s| !s.is_empty()) {
            params["model"] = json!(m);
        }
        let result = self.request("thread/start", params).await?;
        thread_id_from_result(&result).ok_or_else(|| {
            AppServerError::Protocol(format!("thread/start missing thread.id: {result}"))
        })
    }

    pub async fn thread_resume(&mut self, thread_id: &str) -> Result<String, AppServerError> {
        let params = json!({
            "threadId": thread_id,
            "approvalPolicy": "never",
        });
        match self.request("thread/resume", params).await {
            Ok(result) => thread_id_from_result(&result)
                .or_else(|| Some(thread_id.to_string()))
                .ok_or_else(|| {
                    AppServerError::Protocol(format!("thread/resume missing thread.id: {result}"))
                }),
            Err(e) if is_missing_thread_error(&e) => {
                Err(AppServerError::SessionNotFound(e.to_string()))
            }
            Err(e) => Err(e),
        }
    }

    /// Start a turn; returns the turn id. Drain stream events until completed.
    pub async fn turn_start(
        &mut self,
        thread_id: &str,
        input: Vec<Value>,
        model: Option<&str>,
        writable_roots: &[PathBuf],
    ) -> Result<String, AppServerError> {
        // Drain stale notifications from prior turns.
        {
            let mut rx = self.notif_rx.lock().await;
            while rx.try_recv().is_ok() {}
        }
        {
            let mut rx = self.user_input_rx.lock().await;
            while rx.try_recv().is_ok() {}
        }

        let mut params = json!({
            "threadId": thread_id,
            "input": input,
            "sandboxPolicy": {
                "type": "workspaceWrite",
                "writableRoots": writable_roots,
            },
        });
        if let Some(m) = model.filter(|s| !s.is_empty()) {
            params["model"] = json!(m);
        }

        let result = self.request("turn/start", params).await?;
        result
            .pointer("/turn/id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                AppServerError::Protocol(format!("turn/start missing turn.id: {result}"))
            })
    }

    /// Next notification or parked user-input while a turn is running.
    pub async fn next_stream_event(&mut self) -> Result<TurnStreamEvent, AppServerError> {
        // Prefer non-blocking poll of both channels, then await either.
        {
            let mut ui = self.user_input_rx.lock().await;
            if let Ok(need) = ui.try_recv() {
                return Ok(TurnStreamEvent::UserInput(need));
            }
        }
        {
            let mut nx = self.notif_rx.lock().await;
            if let Ok(n) = nx.try_recv() {
                return Ok(TurnStreamEvent::Notification(n));
            }
        }

        // Await the first available event without holding both locks.
        let notif_rx = Arc::clone(&self.notif_rx);
        let user_rx = Arc::clone(&self.user_input_rx);
        tokio::select! {
            need = async {
                let mut rx = user_rx.lock().await;
                rx.recv().await
            } => {
                let need = need.ok_or_else(|| {
                    AppServerError::Process("user-input channel closed".into())
                })?;
                Ok(TurnStreamEvent::UserInput(need))
            }
            notif = async {
                let mut rx = notif_rx.lock().await;
                rx.recv().await
            } => {
                let notif = notif.ok_or_else(|| {
                    AppServerError::Process("notification channel closed".into())
                })?;
                Ok(TurnStreamEvent::Notification(notif))
            }
        }
    }

    /// Complete a parked user-input request with a host decision.
    pub async fn complete_user_input(
        &self,
        rpc_id: &Value,
        decision: &UserInputDecision,
    ) -> Result<(), AppServerError> {
        let msg = encode_user_input_rpc(rpc_id, decision);
        self.write_line(&msg).await
    }

    /// Best-effort cancel of an in-flight turn before ending process heat.
    pub async fn turn_interrupt(
        &mut self,
        thread_id: &str,
        turn_id: &str,
    ) -> Result<(), AppServerError> {
        let _ = self
            .request(
                "turn/interrupt",
                json!({
                    "threadId": thread_id,
                    "turnId": turn_id,
                }),
            )
            .await;
        Ok(())
    }

    pub async fn kill(mut self) {
        self.reader.abort();
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, AppServerError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        {
            let mut pend = self.pending.lock().await;
            pend.insert(id, tx);
        }

        let msg = json!({
            "id": id,
            "method": method,
            "params": params,
        });
        self.write_line(&msg).await?;

        match rx.await {
            Ok(result) => result,
            Err(_) => Err(AppServerError::Process(
                "app-server response channel dropped".into(),
            )),
        }
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<(), AppServerError> {
        let mut msg = json!({ "method": method });
        if !params.is_null() {
            msg["params"] = params;
        }
        self.write_line(&msg).await
    }

    async fn write_line(&self, msg: &Value) -> Result<(), AppServerError> {
        let mut line = serde_json::to_string(msg)
            .map_err(|e| AppServerError::Protocol(format!("serialize: {e}")))?;
        line.push('\n');
        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| AppServerError::Process(format!("write app-server stdin: {e}")))?;
        stdin
            .flush()
            .await
            .map_err(|e| AppServerError::Process(format!("flush app-server stdin: {e}")))?;
        Ok(())
    }
}

async fn handle_incoming(
    msg: Value,
    pending: &Arc<Mutex<PendingMap>>,
    stdin: &Arc<Mutex<ChildStdin>>,
    notif_tx: &mpsc::UnboundedSender<Value>,
    user_input_tx: &mpsc::UnboundedSender<UserInputNeed>,
) {
    // Response (has id, no method).
    if msg.get("id").is_some() && msg.get("method").is_none() {
        let id = match request_id_u64(msg.get("id")) {
            Some(id) => id,
            None => return,
        };
        let result = if let Some(err) = msg.get("error") {
            Err(map_rpc_error(err))
        } else if let Some(result) = msg.get("result") {
            Ok(result.clone())
        } else {
            Err(AppServerError::Protocol(format!(
                "response missing result/error: {msg}"
            )))
        };
        if let Some(tx) = pending.lock().await.remove(&id) {
            let _ = tx.send(result);
        }
        return;
    }

    // Server request (method + id).
    if let (Some(method), Some(id)) = (
        msg.get("method").and_then(Value::as_str),
        msg.get("id").cloned(),
    ) {
        if is_user_input_method(method) {
            let params = msg.get("params").cloned().unwrap_or(Value::Null);
            let _ = user_input_tx.send(UserInputNeed { rpc_id: id, params });
            return;
        }

        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let result = if is_ordinary_approval_method(method) {
            auto_allow_approval_result(method, &params)
        } else {
            // Unknown server requests: empty result; do not hang the child.
            json!({})
        };
        let response = json!({ "id": id, "result": result });
        if let Ok(line) = serde_json::to_string(&response) {
            let mut stdin = stdin.lock().await;
            let _ = stdin.write_all(line.as_bytes()).await;
            let _ = stdin.write_all(b"\n").await;
            let _ = stdin.flush().await;
        }
        return;
    }

    // Notification (method, no id).
    if msg.get("method").is_some() {
        let _ = notif_tx.send(msg);
    }
}

fn request_id_u64(id: Option<&Value>) -> Option<u64> {
    match id? {
        Value::Number(n) => n.as_u64().or_else(|| n.as_i64().map(|i| i as u64)),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn thread_id_from_result(result: &Value) -> Option<String> {
    result
        .pointer("/thread/id")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn map_rpc_error(err: &Value) -> AppServerError {
    let message = err
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("app-server error")
        .to_string();
    if looks_like_missing_thread(&message) {
        AppServerError::SessionNotFound(message)
    } else {
        AppServerError::Protocol(message)
    }
}

fn is_missing_thread_error(err: &AppServerError) -> bool {
    match err {
        AppServerError::SessionNotFound(_) => true,
        AppServerError::Protocol(m) | AppServerError::Process(m) => looks_like_missing_thread(m),
        AppServerError::Spawn(_) => false,
    }
}

fn looks_like_missing_thread(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("no rollout found")
        || lower.contains("not found")
        || lower.contains("unknown thread")
        || lower.contains("no such thread")
        || lower.contains("no conversation")
}
