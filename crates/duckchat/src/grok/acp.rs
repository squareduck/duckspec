//! A per-turn ACP (Agent Client Protocol) client over the grok child's stdio.
//!
//! ACP is JSON-RPC 2.0, one message per line. A turn's lifecycle is
//! `initialize` → (`session/new` when no prior id, else `session/load`) →
//! `session/prompt`. Requests carry an `id` and are answered by an `id`-matched
//! response; `session/update` messages are notifications (the event stream);
//! any agent→client request (e.g. a permission prompt) is auto-answered so the
//! turn never deadlocks. The child is spawned `--always-approve`, so permission
//! round-trips should not occur — the auto-answer is a safety net, and a real
//! interactive permission bridge is deferred (see the design's risks).

use std::path::Path;
use std::pin::Pin;

use serde_json::{Value, json};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use crate::cancel::CancelToken;
use crate::error::Error;
use crate::event::AgentEvent;
use crate::request::ReasoningMode;

use super::event::map_update;

/// The ACP JSON-RPC protocol version this client speaks.
const PROTOCOL_VERSION: u64 = 1;

/// A boxed async line source / sink. Boxing keeps [`AcpTurn`] non-generic over
/// the transport so the real spawn path and the in-memory test peer share one
/// type.
type Reader = Pin<Box<dyn AsyncBufRead + Send + Unpin>>;
type Writer = Pin<Box<dyn AsyncWrite + Send + Unpin>>;

/// One grok agent turn: the child process plus a line-delimited JSON-RPC
/// transport over its stdio.
pub struct AcpTurn {
    /// The spawned `grok agent stdio` child. `None` in tests, which drive a
    /// scripted in-memory peer instead of a live process.
    child: Option<Child>,
    writer: Writer,
    reader: Reader,
    next_id: u64,
}

/// Parsed `initialize` handshake result: whether the agent can resume sessions
/// and the models it advertises (each with its context window).
#[derive(Debug, Clone)]
pub struct InitResult {
    pub load_session: bool,
    pub models: Vec<AcpModel>,
}

/// A model grok advertises in its `modelState.availableModels`.
#[derive(Debug, Clone)]
pub struct AcpModel {
    pub id: String,
    pub name: String,
    pub context_window: Option<usize>,
}

/// Result of a completed `session/prompt`.
#[derive(Debug, Clone)]
pub struct PromptResult {
    /// Why the turn stopped, e.g. `"end_turn"`. Anything else is surfaced as an
    /// error by higher layers.
    pub stop_reason: Option<String>,
}

impl AcpTurn {
    /// Spawn `grok agent --always-approve stdio` in `cwd` and wrap its stdio as
    /// a JSON-RPC transport. Does not yet run the handshake — call
    /// [`AcpTurn::initialize`] next.
    pub async fn spawn(cwd: &Path) -> Result<Self, Error> {
        Self::spawn_with(super::spawn::grok_command(), cwd).await
    }

    /// Like [`AcpTurn::spawn`] but over a caller-supplied base command, which
    /// this appends the `agent --always-approve stdio` args and stdio wiring to
    /// before spawning. The real path passes `spawn::grok_command()`; tests pass
    /// a command pointing at a missing binary to exercise graceful spawn
    /// failure without touching the environment.
    pub async fn spawn_with(mut cmd: Command, cwd: &Path) -> Result<Self, Error> {
        use std::process::Stdio;

        cmd.arg("agent")
            .arg("--always-approve")
            .arg("stdio")
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| Error::Spawn(format!("failed to spawn grok: {e}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Process("no stdin on grok subprocess".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Process("no stdout on grok subprocess".into()))?;

        Ok(Self {
            child: Some(child),
            writer: Box::pin(stdin),
            reader: Box::pin(BufReader::new(stdout)),
            next_id: 1,
        })
    }

    /// Run the `initialize` handshake, returning the agent's resume capability
    /// and advertised models.
    pub async fn initialize(&mut self) -> Result<InitResult, Error> {
        let params = json!({
            "protocolVersion": PROTOCOL_VERSION,
            "clientCapabilities": {},
        });
        let result = self
            .request("initialize", params, &mut noop_update, &CancelToken::new())
            .await?;
        Ok(InitResult::from_result(&result))
    }

    /// Open the session for this turn: `session/new` when `session_id` is
    /// `None`, else `session/load` to resume the given id. Returns the resolved
    /// session id — the one grok assigns for a new session, or the resumed id.
    pub async fn open(&mut self, session_id: Option<&str>, cwd: &Path) -> Result<String, Error> {
        let cwd = cwd.to_string_lossy().into_owned();
        match session_id {
            None => {
                let params = json!({ "cwd": cwd, "mcpServers": [] });
                let result = self
                    .request("session/new", params, &mut noop_update, &CancelToken::new())
                    .await?;
                result
                    .get("sessionId")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .ok_or_else(|| Error::Protocol("session/new returned no sessionId".into()))
            }
            Some(sid) => {
                let params = json!({ "sessionId": sid, "cwd": cwd, "mcpServers": [] });
                // `session/load` may replay history as `session/update`
                // notifications; the caller resumes into an existing transcript
                // so we discard them here.
                self.request("session/load", params, &mut noop_update, &CancelToken::new())
                    .await?;
                Ok(sid.to_string())
            }
        }
    }

    /// Send `session/prompt` and pump the turn to completion, invoking
    /// `on_update` for every `session/update` notification. Returns the stop
    /// reason. Translating updates into agent events is the caller's job.
    ///
    /// `cancel` is checked cooperatively between protocol lines: a flipped flag
    /// kills the child and returns [`Error::Cancelled`].
    pub async fn prompt(
        &mut self,
        session_id: &str,
        text: &str,
        model: &str,
        reasoning: Option<ReasoningMode>,
        on_update: &mut (dyn FnMut(&Value) + Send),
        cancel: &CancelToken,
    ) -> Result<PromptResult, Error> {
        let mut params = json!({
            "sessionId": session_id,
            "prompt": [ { "type": "text", "text": text } ],
        });
        if !model.is_empty() {
            params["model"] = json!(model);
        }
        if let Some(effort) = reasoning.and_then(reasoning_effort) {
            params["reasoningEffort"] = json!(effort);
        }
        let result = self
            .request("session/prompt", params, on_update, cancel)
            .await?;
        Ok(PromptResult {
            stop_reason: result
                .get("stopReason")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        })
    }

    /// Run a prompt turn, translating each `session/update` into a neutral
    /// [`AgentEvent`] and forwarding it onto `events`. A thin wrapper over
    /// [`AcpTurn::prompt`] that supplies the notification sink: it runs
    /// [`map_update`] (folding in the active model's `context_window` for usage
    /// events) and forwards the result. Mapping the returned `stop_reason` onto
    /// `TurnComplete`/`Error` stays with the `run_turn` caller.
    ///
    /// The sink is synchronous, so events are forwarded with `try_send`: a full
    /// bounded channel means the consumer is lagging, and dropping a streamed
    /// delta is preferable to stalling the read loop (which also gates the
    /// prompt response).
    #[allow(clippy::too_many_arguments)] // turn parameters are irreducibly distinct
    pub async fn prompt_events(
        &mut self,
        session_id: &str,
        text: &str,
        model: &str,
        reasoning: Option<ReasoningMode>,
        context_window: Option<usize>,
        events: &mpsc::Sender<AgentEvent>,
        cancel: &CancelToken,
    ) -> Result<PromptResult, Error> {
        let mut on_update = |params: &Value| {
            if let Some(event) = map_update(params, context_window) {
                let _ = events.try_send(event);
            }
        };
        self.prompt(session_id, text, model, reasoning, &mut on_update, cancel)
            .await
    }

    /// Best-effort cancel: ask grok to stop, then drop/kill the child.
    pub async fn cancel(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
        }
    }

    /// Issue a JSON-RPC request and pump messages until the `id`-matched
    /// response arrives. `session/update` notifications are handed to
    /// `on_update`; agent→client requests are auto-answered so the turn never
    /// deadlocks; responses to other ids are ignored.
    async fn request(
        &mut self,
        method: &str,
        params: Value,
        on_update: &mut (dyn FnMut(&Value) + Send),
        cancel: &CancelToken,
    ) -> Result<Value, Error> {
        let id = self.next_id;
        self.next_id += 1;
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .await?;

        loop {
            let msg = self.read_message().await?;
            // Cooperative cancellation: checked between protocol lines, matching
            // the Claude path. A flipped flag kills the child and unwinds.
            if cancel.is_cancelled() {
                self.cancel().await;
                return Err(Error::Cancelled);
            }
            let has_method = msg.get("method").is_some();
            match (has_method, msg.get("id")) {
                // A response: `id` present, no `method`.
                (false, Some(resp_id)) => {
                    if resp_id == &json!(id) {
                        if let Some(err) = msg.get("error") {
                            return Err(Error::Protocol(format!(
                                "grok {method} failed: {err}"
                            )));
                        }
                        return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
                    }
                    // Response to an id we're not waiting on — ignore.
                }
                // An agent→client request: `id` and `method` both present.
                (true, Some(req_id)) => {
                    self.answer_request(req_id.clone()).await?;
                }
                // A notification: `method`, no `id`.
                (true, None) => {
                    let notif = msg.get("method").and_then(Value::as_str).unwrap_or("");
                    if notif == "session/update"
                        && let Some(p) = msg.get("params")
                    {
                        on_update(p);
                    }
                }
                (false, None) => { /* malformed — ignore */ }
            }
        }
    }

    /// Auto-answer an agent→client request with a null result. Since the child
    /// runs `--always-approve`, permission requests should not arrive; this
    /// keeps the loop from deadlocking if one does.
    async fn answer_request(&mut self, id: Value) -> Result<(), Error> {
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": Value::Null,
        }))
        .await
    }

    async fn write_message(&mut self, msg: &Value) -> Result<(), Error> {
        let mut line = serde_json::to_string(msg)
            .map_err(|e| Error::Protocol(format!("failed to encode json-rpc: {e}")))?;
        line.push('\n');
        self.writer.write_all(line.as_bytes()).await?;
        self.writer.flush().await?;
        Ok(())
    }

    async fn read_message(&mut self) -> Result<Value, Error> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).await?;
        if n == 0 {
            return Err(Error::Protocol("grok closed the connection".into()));
        }
        serde_json::from_str(line.trim_end())
            .map_err(|e| Error::Protocol(format!("malformed json-rpc line: {e}")))
    }
}

/// No-op notification sink for requests whose updates are discarded
/// (`initialize`, `session/new`, `session/load`).
fn noop_update(_: &Value) {}

/// Map duckchat's neutral reasoning mode onto grok's reasoning-effort string.
/// `Off` yields `None` (omit the knob entirely).
fn reasoning_effort(mode: ReasoningMode) -> Option<&'static str> {
    match mode {
        ReasoningMode::Off => None,
        ReasoningMode::Low => Some("low"),
        ReasoningMode::Medium => Some("medium"),
        ReasoningMode::High => Some("high"),
    }
}

impl InitResult {
    fn from_result(v: &Value) -> Self {
        let load_session = v
            .pointer("/agentCapabilities/loadSession")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        // grok nests `modelState` inside the handshake result's `_meta`; accept
        // the un-nested form too so a protocol tweak that promotes it still
        // resolves.
        let models = v
            .pointer("/_meta/modelState/availableModels")
            .or_else(|| v.pointer("/modelState/availableModels"))
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(AcpModel::from_value).collect())
            .unwrap_or_default();
        Self {
            load_session,
            models,
        }
    }
}

impl AcpModel {
    fn from_value(v: &Value) -> Option<Self> {
        let id = v
            .get("modelId")
            .or_else(|| v.get("id"))
            .and_then(Value::as_str)?
            .to_string();
        let name = v
            .get("name")
            .or_else(|| v.get("displayName"))
            .and_then(Value::as_str)
            .unwrap_or(&id)
            .to_string();
        // Each advertised model carries its window under its own `_meta`; fall
        // back to a top-level field for forward compatibility.
        let context_window = v
            .pointer("/_meta/totalContextTokens")
            .or_else(|| v.get("totalContextTokens"))
            .and_then(Value::as_u64)
            .map(|n| n as usize);
        Some(Self {
            id,
            name,
            context_window,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll};

    /// A `Writer` that captures everything the client sends, so a test can read
    /// back the request method and params.
    struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

    impl AsyncWrite for CaptureWriter {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }
        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    /// Build an [`AcpTurn`] over a scripted in-memory peer: `responses` are the
    /// JSON-RPC lines grok would send back; the returned buffer captures what
    /// the client writes. No live process is involved.
    fn scripted(responses: &str) -> (AcpTurn, Arc<Mutex<Vec<u8>>>) {
        let written = Arc::new(Mutex::new(Vec::new()));
        let turn = AcpTurn {
            child: None,
            writer: Box::pin(CaptureWriter(written.clone())),
            reader: Box::pin(BufReader::new(Cursor::new(responses.as_bytes().to_vec()))),
            next_id: 1,
        };
        (turn, written)
    }

    fn last_request(written: &Arc<Mutex<Vec<u8>>>) -> Value {
        let bytes = written.lock().unwrap().clone();
        let line = String::from_utf8(bytes).unwrap();
        serde_json::from_str(line.trim()).unwrap()
    }

    /// @spec harness/grok Session lifecycle and resume: A turn without a prior session opens a new session
    #[tokio::test]
    async fn open_without_prior_session_starts_new() {
        let (mut turn, written) =
            scripted("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"sessionId\":\"sess-new\"}}\n");

        let sid = turn.open(None, Path::new("/proj")).await.unwrap();

        // Reports the id grok assigned to the fresh session.
        assert_eq!(sid, "sess-new");
        // Opened it by requesting session/new, not a resume.
        let req = last_request(&written);
        assert_eq!(req["method"], "session/new");
        assert_eq!(req["params"]["cwd"], "/proj");
    }

    #[tokio::test]
    async fn initialize_parses_models_from_meta_nested_handshake() {
        // grok's real handshake nests `modelState` under the result's `_meta`,
        // and each model's window under its own `_meta.totalContextTokens`.
        // Parsing must resolve models and windows through that nesting.
        let response = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\
            \"protocolVersion\":1,\
            \"agentCapabilities\":{\"loadSession\":true},\
            \"_meta\":{\"modelState\":{\"currentModelId\":\"grok-4.5\",\"availableModels\":[\
                {\"modelId\":\"grok-4.5\",\"name\":\"Grok 4.5\",\"_meta\":{\"totalContextTokens\":500000}},\
                {\"modelId\":\"grok-composer-2.5-fast\",\"name\":\"Composer 2.5\",\"_meta\":{\"totalContextTokens\":200000}}\
            ]}}}}\n";
        let (mut turn, _written) = scripted(response);

        let init = turn.initialize().await.unwrap();

        assert!(init.load_session);
        let ids: Vec<&str> = init.models.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["grok-4.5", "grok-composer-2.5-fast"]);
        assert_eq!(init.models[0].name, "Grok 4.5");
        assert_eq!(init.models[0].context_window, Some(500_000));
        assert_eq!(init.models[1].context_window, Some(200_000));
    }

    /// @spec harness/grok Session lifecycle and resume: A turn with a prior session id resumes that session
    #[tokio::test]
    async fn open_with_prior_session_resumes_it() {
        let (mut turn, written) =
            scripted("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n");

        let sid = turn
            .open(Some("sess-123"), Path::new("/proj"))
            .await
            .unwrap();

        // Resumes and reports the same id it was given.
        assert_eq!(sid, "sess-123");
        // Opened it by resuming that exact id via session/load.
        let req = last_request(&written);
        assert_eq!(req["method"], "session/load");
        assert_eq!(req["params"]["sessionId"], "sess-123");
    }
}
