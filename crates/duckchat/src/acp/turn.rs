//! An ACP (Agent Client Protocol) client over a long-lived agent child's stdio.
//!
//! ACP is JSON-RPC 2.0, one message per line. A process-hot runtime holds one
//! [`AcpTurn`] across turns: `initialize` once, then each turn runs
//! (`session/new` when no prior id, else `session/load`) → `session/prompt`.
//! Requests carry an `id` and are answered by an `id`-matched response;
//! `session/update` messages are notifications (the event stream); any
//! agent→client request (e.g. a permission prompt) is auto-answered so the
//! turn never deadlocks.

use std::path::Path;
use std::pin::Pin;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process::Child;
use tokio::sync::mpsc;

use crate::cancel::CancelToken;
use crate::cwd::normalize_cwd;
use crate::error::{self, Error};
use crate::event::AgentEvent;
use crate::request::ReasoningMode;

use super::event::map_update;
use super::launch::AgentLaunch;

/// The ACP JSON-RPC protocol version this client speaks.
const PROTOCOL_VERSION: u64 = 1;

/// A boxed async line source / sink. Boxing keeps [`AcpTurn`] non-generic over
/// the transport so the real spawn path and the in-memory test peer share one
/// type.
type Reader = Pin<Box<dyn AsyncBufRead + Send + Unpin>>;
type Writer = Pin<Box<dyn AsyncWrite + Send + Unpin>>;

/// One agent process (or scripted peer): a line-delimited JSON-RPC transport
/// over its stdio. Held across turns when process-hot.
pub struct AcpTurn {
    /// The spawned agent child. `None` in tests, which drive a scripted
    /// in-memory peer instead of a live process.
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

/// A model an agent advertises in its `modelState.availableModels`.
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
    /// When the agent rebinds the session id during the turn (e.g. provisional
    /// open → durable native id), the rebound id. `None` when open's id stands.
    pub session_id: Option<String>,
}

impl AcpTurn {
    /// Wrap an existing transport without a live child (tests / scripted peers).
    #[cfg(test)]
    pub(crate) fn from_transport(writer: Writer, reader: Reader) -> Self {
        Self {
            child: None,
            writer,
            reader,
            next_id: 1,
        }
    }

    /// True when the child is still running. Childless test peers are always
    /// considered alive until the runtime drops them.
    pub(crate) fn process_alive(&mut self) -> bool {
        match self.child.as_mut() {
            None => true,
            Some(child) => matches!(child.try_wait(), Ok(None)),
        }
    }

    /// Spawn the launch-supplied agent command in `cwd` and wrap its stdio as a
    /// JSON-RPC transport. Does not yet run the handshake — call
    /// [`AcpTurn::initialize`] next.
    ///
    /// The launch's argv is used **as-is**: no harness-specific flags are
    /// appended. Only `current_dir`, stdio pipes, and `kill_on_drop` are set.
    pub async fn spawn_with(launch: &AgentLaunch, cwd: &Path) -> Result<Self, Error> {
        use std::process::Stdio;

        let mut cmd = launch.command();
        cmd.current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| Error::Spawn(format!("failed to spawn agent: {e}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Process("no stdin on agent subprocess".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Process("no stdout on agent subprocess".into()))?;

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
    /// session id — the one the agent assigns for a new session, or the resumed
    /// id.
    ///
    /// `cwd` is normalized before it is sent so create and resume share one
    /// on-disk key (trailing-slash variants otherwise fork the session store).
    /// A `session/load` that fails because the path is gone maps to
    /// [`Error::SessionNotFound`] so the caller can drop the id and retry.
    pub async fn open(&mut self, session_id: Option<&str>, cwd: &Path) -> Result<String, Error> {
        let cwd = normalize_cwd(cwd).to_string_lossy().into_owned();
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
    /// `content` is the multi-block ACP prompt array (text and/or image
    /// blocks). Optional `reasoning` sets `reasoningEffort` when present
    /// (Grok-style knobs; harnesses that do not support it simply omit it).
    /// `cancel` is checked cooperatively between protocol lines: a flipped flag
    /// kills the child and returns [`Error::Cancelled`].
    pub async fn prompt(
        &mut self,
        session_id: &str,
        content: &[Value],
        model: &str,
        reasoning: Option<ReasoningMode>,
        on_update: &mut (dyn FnMut(&Value) + Send),
        cancel: &CancelToken,
    ) -> Result<PromptResult, Error> {
        let mut params = json!({
            "sessionId": session_id,
            "prompt": content,
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
            session_id: result
                .get("sessionId")
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
        content: &[Value],
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
        self.prompt(session_id, content, model, reasoning, &mut on_update, cancel)
            .await
    }

    /// Best-effort cancel: kill the agent child.
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
            // Cooperative cancellation while waiting for the next line, so a
            // hung peer does not ignore cancel until the next message arrives.
            let msg = self.read_message_cancellable(cancel).await?;
            let has_method = msg.get("method").is_some();
            match (has_method, msg.get("id")) {
                // A response: `id` present, no `method`.
                (false, Some(resp_id)) => {
                    if resp_id == &json!(id) {
                        if let Some(err) = msg.get("error") {
                            // Load or first-prompt resume may report a dead session.
                            if (method == "session/load" || method == "session/prompt")
                                && error::rpc_error_is_session_not_found(err)
                            {
                                return Err(Error::SessionNotFound);
                            }
                            return Err(Error::Protocol(format!(
                                "agent {method} failed: {err}"
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

    /// Read one JSON-RPC line, polling `cancel` while blocked.
    async fn read_message_cancellable(&mut self, cancel: &CancelToken) -> Result<Value, Error> {
        loop {
            if cancel.is_cancelled() {
                self.cancel().await;
                return Err(Error::Cancelled);
            }
            match tokio::time::timeout(Duration::from_millis(25), self.read_message()).await {
                Ok(result) => return result,
                Err(_elapsed) => continue,
            }
        }
    }

    /// Auto-answer an agent→client request with a null result so a headless
    /// host never deadlocks waiting on UI for permission/question prompts.
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
            return Err(Error::Protocol("agent closed the connection".into()));
        }
        serde_json::from_str(line.trim_end())
            .map_err(|e| Error::Protocol(format!("malformed json-rpc line: {e}")))
    }
}

/// No-op notification sink for requests whose updates are discarded
/// (`initialize`, `session/new`, `session/load`).
fn noop_update(_: &Value) {}

/// Map duckchat's neutral reasoning mode onto an optional ACP `reasoningEffort`
/// string. `Off` yields `None` (omit the knob entirely).
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
        // Agents may nest `modelState` inside the handshake result's `_meta`;
        // accept the un-nested form too so a protocol tweak that promotes it
        // still resolves.
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
    use tokio::process::Command;

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
    /// JSON-RPC lines the agent would send back; the returned buffer captures
    /// what the client writes. No live process is involved.
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

    /// @spec harness/acp-client Session open and resume: A turn without a prior session id opens a new session and surfaces the id
    #[tokio::test]
    async fn open_without_prior_session_starts_new() {
        let (mut turn, written) =
            scripted("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"sessionId\":\"sess-new\"}}\n");

        let sid = turn.open(None, Path::new("/proj")).await.unwrap();

        // Reports the id the agent assigned to the fresh session.
        assert_eq!(sid, "sess-new");
        // Opened it by requesting session/new, not a resume.
        let req = last_request(&written);
        assert_eq!(req["method"], "session/new");
        assert_eq!(req["params"]["cwd"], "/proj");
    }

    #[tokio::test]
    async fn initialize_parses_models_from_meta_nested_handshake() {
        // Real handshake nests `modelState` under the result's `_meta`, and
        // each model's window under its own `_meta.totalContextTokens`.
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

    /// @spec harness/acp-client Session open and resume: A turn with a prior session id resumes that id
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

    /// @spec harness/acp-client Session open and resume: A failed load of a missing session surfaces session-not-found
    #[tokio::test]
    async fn open_load_missing_session_is_session_not_found() {
        // Real cinnabar failure shape: cwd-key mismatch or pruned session file.
        let response = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32603,"data":{"code":"FS_NOT_FOUND","detail":"No such file or directory (os error 2)"},"message":"Path not found."}}
"#;
        let (mut turn, _written) = scripted(response);

        let err = turn
            .open(Some("019f489b-dead"), Path::new("/proj/"))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::SessionNotFound));
        assert!(err.is_session_not_found());
    }

    #[tokio::test]
    async fn open_normalizes_trailing_slash_on_cwd() {
        let (mut turn, written) =
            scripted("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"sessionId\":\"s1\"}}\n");

        let _ = turn.open(None, Path::new("/no/such/proj/path/")).await;

        let req = last_request(&written);
        assert_eq!(req["method"], "session/new");
        // Missing path still strips trailing separators so create/resume keys match.
        assert_eq!(req["params"]["cwd"], "/no/such/proj/path");
    }

    /// @spec harness/acp-client Launch-parameterized agent process: The client spawns the launch-supplied agent command
    #[tokio::test]
    async fn client_spawns_launch_supplied_agent_command() {
        // Script records its argv then exits. The client must spawn this
        // command as-is (no extra harness flags appended after our marker).
        let dir = std::env::temp_dir().join(format!(
            "duckchat-acp-launch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let argv_path = dir.join("argv.txt");
        let script_path = dir.join("fake-agent.sh");
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$0\" \"$@\" > '{}'\n",
            argv_path.display()
        );
        std::fs::write(&script_path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script_path, perms).unwrap();
        }

        let script_owned = script_path.clone();
        let launch = AgentLaunch::new(move || {
            let mut cmd = Command::new(&script_owned);
            cmd.arg("--agent-flag").arg("stdio");
            cmd
        });

        // Spawn succeeds (stdio wired); child exits immediately so initialize
        // will fail with a closed connection — that is fine. We only care that
        // the recorded argv is exactly the launch-supplied program + args.
        let spawn_result = AcpTurn::spawn_with(&launch, &dir).await;
        assert!(
            spawn_result.is_ok(),
            "spawn should succeed: {}",
            spawn_result.as_ref().err().map(|e| e.to_string()).unwrap_or_default()
        );
        // Give the shell a moment to write the argv file.
        for _ in 0..50 {
            if argv_path.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let recorded = std::fs::read_to_string(&argv_path).expect("argv file written");
        let lines: Vec<&str> = recorded.lines().collect();
        assert_eq!(
            lines,
            [
                script_path.to_str().unwrap(),
                "--agent-flag",
                "stdio",
            ],
            "client must not append harness flags to the launch argv"
        );
        drop(spawn_result);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
