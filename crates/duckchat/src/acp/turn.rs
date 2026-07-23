//! An ACP (Agent Client Protocol) client over a long-lived agent child's stdio.
//!
//! ACP is JSON-RPC 2.0, one message per line. A process-hot runtime holds one
//! [`AcpTurn`] across turns: `initialize` once, then each turn runs
//! (`session/new` when no prior id, else `session/load`) → `session/prompt`.
//! Requests carry an `id` and are answered by an `id`-matched response;
//! `session/update` messages are notifications (the event stream). Mid-turn
//! agent→client requests are classified: tool permissions auto-allow, structured
//! questions park for the host, unknown methods complete non-blocking.

use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process::Child;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::cancel::CancelToken;
use crate::cwd::normalize_cwd;
use crate::error::{self, Error};
use crate::event::{
    AgentEvent, PendingUserChoices, UserChoiceAnswer, UserChoiceOption, UserChoiceRequest,
};
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
            .request(
                "initialize",
                params,
                &mut noop_update,
                &CancelToken::new(),
                ClientRequestHost::Auto,
            )
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
                    .request(
                        "session/new",
                        params,
                        &mut noop_update,
                        &CancelToken::new(),
                        ClientRequestHost::Auto,
                    )
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
                self.request(
                    "session/load",
                    params,
                    &mut noop_update,
                    &CancelToken::new(),
                    ClientRequestHost::Auto,
                )
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
            .request(
                "session/prompt",
                params,
                on_update,
                cancel,
                ClientRequestHost::Auto,
            )
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
    ///
    /// Mid-turn structured questions park on `pending_choices` and emit
    /// [`AgentEvent::UserChoiceRequest`] for the host.
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
        pending_choices: &Arc<PendingUserChoices>,
    ) -> Result<PromptResult, Error> {
        let mut on_update = |params: &Value| {
            if let Some(event) = map_update(params, context_window) {
                let _ = events.try_send(event);
            }
        };
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
        let host = ClientRequestHost::Interactive {
            events,
            pending: pending_choices,
        };
        let result = self
            .request("session/prompt", params, &mut on_update, cancel, host)
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

    /// Best-effort cancel: kill the agent child.
    pub async fn cancel(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
        }
    }

    /// Issue a JSON-RPC request and pump messages until the `id`-matched
    /// response arrives. `session/update` notifications are handed to
    /// `on_update`; agent→client requests are classified via `host`.
    async fn request(
        &mut self,
        method: &str,
        params: Value,
        on_update: &mut (dyn FnMut(&Value) + Send),
        cancel: &CancelToken,
        host: ClientRequestHost<'_>,
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
            let msg = match self.read_message_cancellable(cancel).await {
                Ok(m) => m,
                Err(Error::Cancelled) => {
                    if let ClientRequestHost::Interactive { pending, .. } = host {
                        pending.cancel_all();
                    }
                    return Err(Error::Cancelled);
                }
                Err(e) => return Err(e),
            };
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
                            return Err(Error::Protocol(format!("agent {method} failed: {err}")));
                        }
                        return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
                    }
                    // Response to an id we're not waiting on — ignore.
                }
                // An agent→client request: `id` and `method` both present.
                (true, Some(req_id)) => {
                    let agent_method = msg.get("method").and_then(Value::as_str).unwrap_or("");
                    let params = msg.get("params").cloned().unwrap_or(Value::Null);
                    self.handle_agent_request(req_id.clone(), agent_method, &params, cancel, host)
                        .await?;
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

    /// Classify and complete an agent→client request.
    async fn handle_agent_request(
        &mut self,
        req_id: Value,
        method: &str,
        params: &Value,
        cancel: &CancelToken,
        host: ClientRequestHost<'_>,
    ) -> Result<(), Error> {
        match host {
            ClientRequestHost::Auto => {
                // Oneshot / headless: never park on host UI.
                self.auto_complete_agent_request(req_id, method, params)
                    .await
            }
            ClientRequestHost::Interactive { events, pending } => {
                self.interactive_agent_request(req_id, method, params, cancel, events, pending)
                    .await
            }
        }
    }

    /// Complete without host UI (oneshot safety + unknown methods).
    async fn auto_complete_agent_request(
        &mut self,
        req_id: Value,
        method: &str,
        params: &Value,
    ) -> Result<(), Error> {
        let result = match classify_agent_request(method, params) {
            AgentRequestKind::ToolPermission { allow_option_id } => {
                permission_selected_result(&allow_option_id)
            }
            AgentRequestKind::UserChoice { .. } | AgentRequestKind::Unknown => Value::Null,
        };
        self.write_result(req_id, result).await
    }

    /// Main path: auto-allow tool permissions; park structured questions.
    async fn interactive_agent_request(
        &mut self,
        req_id: Value,
        method: &str,
        params: &Value,
        cancel: &CancelToken,
        events: &mpsc::Sender<AgentEvent>,
        pending: &Arc<PendingUserChoices>,
    ) -> Result<(), Error> {
        match classify_agent_request(method, params) {
            AgentRequestKind::ToolPermission { allow_option_id } => {
                self.write_result(req_id, permission_selected_result(&allow_option_id))
                    .await
            }
            AgentRequestKind::UserChoice {
                prompt,
                options,
                wire,
            } => {
                let (correlation_id, rx) = pending.park();
                let allow_cancel = true;
                let _ = events.try_send(AgentEvent::UserChoiceRequest(UserChoiceRequest {
                    correlation_id,
                    prompt,
                    options,
                    allow_cancel,
                }));
                let answer = await_choice_answer(rx, cancel, pending, correlation_id).await;
                pending.forget(correlation_id);
                let result = encode_choice_result(&wire, &answer);
                self.write_result(req_id, result).await?;
                if cancel.is_cancelled() {
                    self.cancel().await;
                    return Err(Error::Cancelled);
                }
                Ok(())
            }
            AgentRequestKind::Unknown => self.write_result(req_id, Value::Null).await,
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

    async fn write_result(&mut self, id: Value, result: Value) -> Result<(), Error> {
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
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
        loop {
            let mut line = String::new();
            let n = self.reader.read_line(&mut line).await?;
            if n == 0 {
                return Err(Error::Protocol("agent closed the connection".into()));
            }
            let trimmed = line.trim_end();
            // Blank lines and non-object noise on the agent stdout pipe
            // (progress, leaked tool chatter, shell banner) are not JSON-RPC.
            // Skip them rather than failing the turn — agents occasionally
            // pollute NDJSON. Real messages are always a single `{…}` object.
            if trimmed.is_empty() || !trimmed.starts_with('{') {
                continue;
            }
            return serde_json::from_str(trimmed).map_err(|e| {
                let preview = truncate_for_error(trimmed, 120);
                Error::Protocol(format!("malformed json-rpc line: {e}; line={preview}"))
            });
        }
    }
}

/// Shorten a bad stdout line for inclusion in a protocol error message.
fn truncate_for_error(s: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max_chars {
            out.push('…');
            break;
        }
        // Keep the preview single-line and readable in system error toasts.
        if ch == '\n' || ch == '\r' {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
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

/// How agent→client requests are completed during a JSON-RPC pump.
#[derive(Clone, Copy)]
enum ClientRequestHost<'a> {
    /// Never park: auto-allow tool permissions, null for questions (oneshot).
    Auto,
    /// Main path: emit and park structured choices for the host.
    Interactive {
        events: &'a mpsc::Sender<AgentEvent>,
        pending: &'a Arc<PendingUserChoices>,
    },
}

/// Classification of an agent→client method.
#[derive(Debug)]
enum AgentRequestKind {
    /// `session/request_permission` with only allow/reject kinds.
    ToolPermission {
        allow_option_id: String,
    },
    /// Structured product question (Grok ask-user, product-labeled permission).
    UserChoice {
        prompt: Option<String>,
        options: Vec<UserChoiceOption>,
        wire: ChoiceWire,
    },
    Unknown,
}

/// How to encode a host answer back onto the agent wire.
#[derive(Clone, Debug)]
enum ChoiceWire {
    /// ACP `session/request_permission` selected / cancelled outcome.
    Permission,
    /// Grok ask-user extension response (`_x.ai/ask_user_question` / alias).
    AskUserQuestion {
        /// Question text used as the `answers` map key.
        question_text: String,
        /// Options at park time (for resolving option id → label).
        options: Vec<UserChoiceOption>,
    },
}

fn is_permission_kind(kind: &str) -> bool {
    matches!(
        kind,
        "allow_once" | "allow_always" | "reject_once" | "reject_always"
    )
}

fn permission_options(params: &Value) -> Option<&Vec<Value>> {
    params.get("options").and_then(Value::as_array)
}

fn options_are_only_permission_kinds(options: &[Value]) -> bool {
    !options.is_empty()
        && options.iter().all(|o| {
            o.get("kind")
                .and_then(Value::as_str)
                .is_some_and(is_permission_kind)
        })
}

fn first_allow_option_id(options: &[Value]) -> Option<String> {
    options.iter().find_map(|o| {
        let kind = o.get("kind")?.as_str()?;
        if kind.starts_with("allow_") {
            o.get("optionId")
                .or_else(|| o.get("option_id"))
                .and_then(Value::as_str)
                .map(str::to_string)
        } else {
            None
        }
    })
}

fn permission_selected_result(option_id: &str) -> Value {
    json!({
        "outcome": {
            "outcome": "selected",
            "optionId": option_id,
        }
    })
}

fn permission_cancelled_result() -> Value {
    json!({
        "outcome": {
            "outcome": "cancelled",
        }
    })
}

fn encode_choice_result(wire: &ChoiceWire, answer: &UserChoiceAnswer) -> Value {
    match (wire, answer) {
        (ChoiceWire::Permission, UserChoiceAnswer::Selected { option_id }) => {
            permission_selected_result(option_id)
        }
        // Custom freeform: carry text as optionId so Claude parent→allow maps free text.
        (ChoiceWire::Permission, UserChoiceAnswer::Custom { text }) => {
            permission_selected_result(text)
        }
        (ChoiceWire::Permission, UserChoiceAnswer::Cancelled) => permission_cancelled_result(),
        (
            ChoiceWire::AskUserQuestion {
                question_text,
                options,
            },
            UserChoiceAnswer::Selected { option_id },
        ) => {
            let label = super::ask_user::label_for_selection(options, option_id);
            super::ask_user::encode_selected(question_text, &label)
        }
        (ChoiceWire::AskUserQuestion { question_text, .. }, UserChoiceAnswer::Custom { text }) => {
            super::ask_user::encode_selected(question_text, text)
        }
        (ChoiceWire::AskUserQuestion { .. }, UserChoiceAnswer::Cancelled) => {
            super::ask_user::encode_cancelled()
        }
    }
}

fn product_options_from_permission(options: &[Value]) -> Vec<UserChoiceOption> {
    options
        .iter()
        .filter_map(|o| {
            let id = o
                .get("optionId")
                .or_else(|| o.get("option_id"))
                .and_then(Value::as_str)?
                .to_string();
            let label = o
                .get("name")
                .or_else(|| o.get("label"))
                .and_then(Value::as_str)
                .unwrap_or(&id)
                .to_string();
            Some(UserChoiceOption { id, label })
        })
        .collect()
}

/// Question text for product `session/request_permission` choices.
/// Claude puts the AskUserQuestion text on `toolCall.title`.
fn permission_choice_prompt(params: &Value) -> Option<String> {
    let title = params
        .pointer("/toolCall/title")
        .or_else(|| params.pointer("/tool_call/title"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    Some(title.to_string())
}

fn classify_agent_request(method: &str, params: &Value) -> AgentRequestKind {
    match method {
        "session/request_permission" => {
            let Some(options) = permission_options(params) else {
                return AgentRequestKind::Unknown;
            };
            if options_are_only_permission_kinds(options) {
                match first_allow_option_id(options) {
                    Some(allow_option_id) => AgentRequestKind::ToolPermission { allow_option_id },
                    None => AgentRequestKind::Unknown,
                }
            } else {
                let opts = product_options_from_permission(options);
                if opts.is_empty() {
                    AgentRequestKind::Unknown
                } else {
                    AgentRequestKind::UserChoice {
                        prompt: permission_choice_prompt(params),
                        options: opts,
                        wire: ChoiceWire::Permission,
                    }
                }
            }
        }
        m if super::ask_user::is_ask_user_method(m) => {
            let (prompt, options) = super::ask_user::decode_options(params);
            if options.is_empty() {
                AgentRequestKind::Unknown
            } else {
                let question_text = prompt.clone().unwrap_or_default();
                AgentRequestKind::UserChoice {
                    prompt,
                    options: options.clone(),
                    wire: ChoiceWire::AskUserQuestion {
                        question_text,
                        options,
                    },
                }
            }
        }
        _ => AgentRequestKind::Unknown,
    }
}

async fn await_choice_answer(
    mut rx: oneshot::Receiver<UserChoiceAnswer>,
    cancel: &CancelToken,
    pending: &PendingUserChoices,
    correlation_id: u64,
) -> UserChoiceAnswer {
    loop {
        if cancel.is_cancelled() {
            pending.forget(correlation_id);
            return UserChoiceAnswer::Cancelled;
        }
        tokio::select! {
            biased;
            ans = &mut rx => {
                return ans.unwrap_or(UserChoiceAnswer::Cancelled);
            }
            _ = tokio::time::sleep(Duration::from_millis(25)) => {}
        }
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
    use tokio::io::AsyncWriteExt;
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

    /// Non-object stdout lines (noise that matches the observed grok
    /// "trailing characters at column 4" / "expected value at column 1"
    /// failures) must not fail the turn — only the following JSON-RPC object
    /// is consumed.
    #[tokio::test]
    async fn read_message_skips_non_object_stdout_noise() {
        // Lines that previously hard-failed the turn in production:
        // - `2.0}` → trailing characters at column 4
        // - bare text → expected value at column 1
        // - blank lines
        let noise_then_ok = "\n\
            2.0}\n\
            cargo test noise\n\
            123x\n\
            {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"sessionId\":\"after-noise\"}}\n";
        let (mut turn, _written) = scripted(noise_then_ok);

        let sid = turn.open(None, Path::new("/proj")).await.unwrap();
        assert_eq!(sid, "after-noise");
    }

    #[tokio::test]
    async fn read_message_rejects_malformed_object_with_line_preview() {
        let (mut turn, _written) = scripted("{\"jsonrpc\":\"2.0\" NOT-JSON\n");
        let err = turn.open(None, Path::new("/proj")).await.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("malformed json-rpc line"),
            "expected protocol error, got {msg}"
        );
        assert!(
            msg.contains("line="),
            "error should include a line preview: {msg}"
        );
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
        let (mut turn, written) = scripted("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n");

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
            spawn_result
                .as_ref()
                .err()
                .map(|e| e.to_string())
                .unwrap_or_default()
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
            [script_path.to_str().unwrap(), "--agent-flag", "stdio",],
            "client must not append harness flags to the launch argv"
        );
        drop(spawn_result);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Mid-turn agent→client request routing ─────────────────────────────

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

    /// Fake peer for mid-turn tests. `on_prompt` is called once after
    /// session/prompt is received; it may write agent→client requests, then
    /// must leave the prompt response to be sent when the test is ready.
    async fn peer_with_midturn<F, Fut>(on_prompt: F) -> (AcpTurn, Arc<Mutex<Vec<Value>>>)
    where
        F: FnOnce(Value, tokio::io::WriteHalf<tokio::io::DuplexStream>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = tokio::io::WriteHalf<tokio::io::DuplexStream>>
            + Send
            + 'static,
    {
        use tokio::io::duplex;

        let (client, server) = duplex(64 * 1024);
        let (server_read, server_write) = tokio::io::split(server);
        let (client_read, client_write) = tokio::io::split(client);
        let written = Arc::new(Mutex::new(Vec::new()));
        let written_peer = written.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(server_read);
            let mut writer = server_write;
            let mut line = String::new();
            let mut on_prompt = Some(on_prompt);
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
                written_peer.lock().unwrap().push(msg.clone());
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
                                "_meta": { "modelState": { "availableModels": [] } }
                            }
                        });
                        if write_line(&mut writer, &reply).await.is_err() {
                            break;
                        }
                    }
                    "session/new" => {
                        let reply = json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": { "sessionId": "mid-sess" }
                        });
                        if write_line(&mut writer, &reply).await.is_err() {
                            break;
                        }
                    }
                    "session/load" => {
                        let reply = json!({ "jsonrpc": "2.0", "id": id, "result": {} });
                        if write_line(&mut writer, &reply).await.is_err() {
                            break;
                        }
                    }
                    "session/prompt" => {
                        if let Some(cb) = on_prompt.take() {
                            writer = cb(id.clone(), writer).await;
                        }
                        let reply = json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": { "stopReason": "end_turn" }
                        });
                        if write_line(&mut writer, &reply).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        });

        let turn = AcpTurn::from_transport(
            Box::pin(client_write),
            Box::pin(BufReader::new(client_read)),
        );
        (turn, written)
    }

    // @spec harness/acp-client Mid-turn tool permission auto-allow: Permission request with only allow/reject kinds is auto-allowed
    #[tokio::test]
    async fn permission_request_with_only_allow_reject_kinds_is_auto_allowed() {
        let (mut turn, written) = peer_with_midturn(|prompt_id, mut writer| async move {
            let _ = prompt_id;
            let req = json!({
                "jsonrpc": "2.0",
                "id": 99,
                "method": "session/request_permission",
                "params": {
                    "sessionId": "mid-sess",
                    "toolCall": { "toolCallId": "t1", "title": "bash" },
                    "options": [
                        { "optionId": "allow-once", "name": "Allow", "kind": "allow_once" },
                        { "optionId": "reject-once", "name": "Reject", "kind": "reject_once" }
                    ]
                }
            });
            write_line(&mut writer, &req).await.unwrap();
            // Wait briefly for client auto-allow before ending the turn.
            tokio::time::sleep(Duration::from_millis(50)).await;
            writer
        })
        .await;

        let (tx, mut rx) = mpsc::channel(16);
        let pending = PendingUserChoices::shared();
        let cancel = CancelToken::new();
        let content = [json!({ "type": "text", "text": "hi" })];
        let result = turn
            .prompt_events("mid-sess", &content, "", None, None, &tx, &cancel, &pending)
            .await
            .expect("prompt completes");
        assert_eq!(result.stop_reason.as_deref(), Some("end_turn"));

        // No host user-choice event.
        while let Ok(ev) = rx.try_recv() {
            assert!(
                !matches!(ev, AgentEvent::UserChoiceRequest(_)),
                "tool permission must not emit UserChoiceRequest"
            );
        }

        let msgs = written.lock().unwrap().clone();
        let allow = msgs
            .iter()
            .find(|m| m.get("id") == Some(&json!(99)) && m.get("result").is_some());
        let allow = allow.expect("client answered permission request");
        assert_eq!(
            allow["result"]["outcome"]["outcome"], "selected",
            "auto-allow must select an option"
        );
        assert_eq!(allow["result"]["outcome"]["optionId"], "allow-once");
    }

    // @spec harness/acp-client Mid-turn user choice: Structured question request surfaces a user-choice event
    #[tokio::test]
    async fn structured_question_request_surfaces_a_user_choice_event() {
        // Live Grok method name (leading underscore).
        let (mut turn, written) = peer_with_midturn(|_prompt_id, mut writer| async move {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 42,
                "method": "_x.ai/ask_user_question",
                "params": {
                    "sessionId": "mid-sess",
                    "toolCallId": "tc-q",
                    "mode": "single",
                    "questions": [{
                        "question": "Pick a path",
                        "options": [
                            { "label": "Ship it", "description": "go" },
                            { "label": "Wait", "description": "hold" }
                        ]
                    }]
                }
            });
            write_line(&mut writer, &req).await.unwrap();
            // Stay open long enough for host answer + client response.
            tokio::time::sleep(Duration::from_millis(200)).await;
            writer
        })
        .await;

        let (tx, mut rx) = mpsc::channel(16);
        let pending = PendingUserChoices::shared();
        let cancel = CancelToken::new();
        let content = [json!({ "type": "text", "text": "hi" })];

        let pending_ans = pending.clone();
        let answerer = tokio::spawn(async move {
            // Wait for the choice event, then answer.
            let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
            loop {
                if let Ok(ev) = rx.try_recv()
                    && let AgentEvent::UserChoiceRequest(req) = ev
                {
                    assert_eq!(req.options.len(), 2);
                    assert_eq!(req.options[0].label, "Ship it");
                    assert_eq!(req.prompt.as_deref(), Some("Pick a path"));
                    pending_ans.answer(
                        req.correlation_id,
                        UserChoiceAnswer::Selected {
                            option_id: "Ship it".into(),
                        },
                    );
                    return;
                }
                if tokio::time::Instant::now() > deadline {
                    panic!("timed out waiting for UserChoiceRequest");
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        let result = turn
            .prompt_events("mid-sess", &content, "", None, None, &tx, &cancel, &pending)
            .await
            .expect("prompt completes after answer");
        assert_eq!(result.stop_reason.as_deref(), Some("end_turn"));
        answerer.await.unwrap();

        let msgs = written.lock().unwrap().clone();
        let reply = msgs
            .iter()
            .find(|m| m.get("id") == Some(&json!(42)) && m.get("result").is_some())
            .expect("ask-user result written");
        assert_eq!(reply["result"]["outcome"], "accepted");
        assert_eq!(reply["result"]["answers"]["Pick a path"], "Ship it");
        assert!(reply["result"]["partial_answers"].is_null());
    }

    // @spec harness/acp-client Mid-turn user choice: Host selected answer completes the pending request
    #[tokio::test]
    async fn host_selected_answer_completes_the_pending_request() {
        let (mut turn, written) = peer_with_midturn(|_prompt_id, mut writer| async move {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "session/request_permission",
                "params": {
                    "sessionId": "mid-sess",
                    "toolCall": { "toolCallId": "q1", "title": "Choose" },
                    "options": [
                        { "optionId": "opt-a", "name": "Alpha", "kind": "custom" },
                        { "optionId": "opt-b", "name": "Beta", "kind": "custom" }
                    ]
                }
            });
            write_line(&mut writer, &req).await.unwrap();
            tokio::time::sleep(Duration::from_millis(200)).await;
            writer
        })
        .await;

        let (tx, mut rx) = mpsc::channel(16);
        let pending = PendingUserChoices::shared();
        let cancel = CancelToken::new();
        let content = [json!({ "type": "text", "text": "hi" })];

        let pending_ans = pending.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(AgentEvent::UserChoiceRequest(req)) = rx.try_recv() {
                    pending_ans.answer(
                        req.correlation_id,
                        UserChoiceAnswer::Selected {
                            option_id: "opt-b".into(),
                        },
                    );
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        turn.prompt_events("mid-sess", &content, "", None, None, &tx, &cancel, &pending)
            .await
            .expect("turn continues after selection");

        let msgs = written.lock().unwrap().clone();
        let reply = msgs
            .iter()
            .find(|m| m.get("id") == Some(&json!(7)) && m.get("result").is_some())
            .expect("permission result written");
        assert_eq!(reply["result"]["outcome"]["outcome"], "selected");
        assert_eq!(reply["result"]["outcome"]["optionId"], "opt-b");
    }

    // @spec harness/acp-client Mid-turn user choice: Host cancel completes the pending request as cancelled
    #[tokio::test]
    async fn host_cancel_completes_the_pending_request_as_cancelled() {
        let (mut turn, written) = peer_with_midturn(|_prompt_id, mut writer| async move {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 8,
                "method": "session/request_permission",
                "params": {
                    "sessionId": "mid-sess",
                    "toolCall": { "toolCallId": "q1", "title": "Choose" },
                    "options": [
                        { "optionId": "opt-a", "name": "Alpha", "kind": "custom" }
                    ]
                }
            });
            write_line(&mut writer, &req).await.unwrap();
            tokio::time::sleep(Duration::from_millis(200)).await;
            writer
        })
        .await;

        let (tx, mut rx) = mpsc::channel(16);
        let pending = PendingUserChoices::shared();
        let cancel = CancelToken::new();
        let content = [json!({ "type": "text", "text": "hi" })];

        let pending_ans = pending.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(AgentEvent::UserChoiceRequest(req)) = rx.try_recv() {
                    pending_ans.answer(req.correlation_id, UserChoiceAnswer::Cancelled);
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        turn.prompt_events("mid-sess", &content, "", None, None, &tx, &cancel, &pending)
            .await
            .expect("turn continues after host cancel");

        let msgs = written.lock().unwrap().clone();
        let reply = msgs
            .iter()
            .find(|m| m.get("id") == Some(&json!(8)) && m.get("result").is_some())
            .expect("cancelled result written");
        assert_eq!(reply["result"]["outcome"]["outcome"], "cancelled");
    }

    // @spec harness/acp-client Mid-turn user choice: Turn cancel completes a pending choice as cancelled
    #[tokio::test]
    async fn turn_cancel_completes_a_pending_choice_as_cancelled() {
        let (mut turn, written) = peer_with_midturn(|_prompt_id, mut writer| async move {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 11,
                "method": "_x.ai/ask_user_question",
                "params": {
                    "questions": [{
                        "question": "Stay?",
                        "options": [{ "label": "Yes" }, { "label": "No" }]
                    }]
                }
            });
            write_line(&mut writer, &req).await.unwrap();
            // Hang until client disconnects / cancels.
            tokio::time::sleep(Duration::from_secs(5)).await;
            writer
        })
        .await;

        let (tx, mut rx) = mpsc::channel(16);
        let pending = PendingUserChoices::shared();
        let cancel = CancelToken::new();
        let cancel2 = cancel.clone();
        let content = [json!({ "type": "text", "text": "hi" })];

        let turn_task = tokio::spawn(async move {
            turn.prompt_events(
                "mid-sess", &content, "", None, None, &tx, &cancel2, &pending,
            )
            .await
        });

        // Wait until the choice is parked, then cancel the turn.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        loop {
            if matches!(rx.try_recv(), Ok(AgentEvent::UserChoiceRequest(_))) {
                break;
            }
            if tokio::time::Instant::now() > deadline {
                panic!("no UserChoiceRequest before cancel");
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        cancel.cancel();

        let err = turn_task.await.unwrap().expect_err("must cancel");
        assert!(matches!(err, Error::Cancelled));

        // Allow the peer to observe the cancelled result write.
        tokio::time::sleep(Duration::from_millis(50)).await;
        let msgs = written.lock().unwrap().clone();
        let reply = msgs
            .iter()
            .find(|m| m.get("id") == Some(&json!(11)) && m.get("result").is_some());
        if let Some(reply) = reply {
            assert_eq!(
                reply["result"]["outcome"], "skip_interview",
                "turn cancel must write skip_interview: {reply}"
            );
        }
    }

    // @spec harness/acp-client Headless and oneshot safety: Oneshot path does not block waiting on a host UI choice
    #[tokio::test]
    async fn oneshot_path_does_not_block_waiting_on_a_host_ui_choice() {
        // Oneshot uses `prompt` (Auto host): a structured question completes
        // with null without parking.
        let (mut turn, written) = peer_with_midturn(|_prompt_id, mut writer| async move {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 55,
                "method": "x.ai/ask_user_question",
                "params": {
                    "questions": [{
                        "question": "Block?",
                        "options": [{ "label": "Yes" }]
                    }]
                }
            });
            write_line(&mut writer, &req).await.unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
            writer
        })
        .await;

        let cancel = CancelToken::new();
        let content = [json!({ "type": "text", "text": "title me" })];
        let mut noop = |_: &Value| {};
        let finished = tokio::time::timeout(
            Duration::from_secs(2),
            turn.prompt("mid-sess", &content, "", None, &mut noop, &cancel),
        )
        .await
        .expect("oneshot must not hang on host UI")
        .expect("prompt ok");
        assert_eq!(finished.stop_reason.as_deref(), Some("end_turn"));

        let msgs = written.lock().unwrap().clone();
        let reply = msgs
            .iter()
            .find(|m| m.get("id") == Some(&json!(55)) && m.get("result").is_some())
            .expect("auto-completed without host");
        assert!(
            reply["result"].is_null(),
            "oneshot null-completes questions"
        );
    }

    #[test]
    fn classify_tool_permission_vs_product_choice() {
        let tool = json!({
            "options": [
                { "optionId": "a", "name": "Allow", "kind": "allow_once" },
                { "optionId": "r", "name": "Reject", "kind": "reject_once" }
            ]
        });
        match classify_agent_request("session/request_permission", &tool) {
            AgentRequestKind::ToolPermission { allow_option_id } => {
                assert_eq!(allow_option_id, "a");
            }
            other => panic!("expected tool permission, got non-tool: {other:?}"),
        }

        let product = json!({
            "options": [
                { "optionId": "x", "name": "Ship", "kind": "custom" }
            ]
        });
        match classify_agent_request("session/request_permission", &product) {
            AgentRequestKind::UserChoice {
                options, prompt, ..
            } => {
                assert_eq!(options[0].id, "x");
                assert_eq!(options[0].label, "Ship");
                assert_eq!(prompt, None, "no toolCall.title → no prompt");
            }
            _ => panic!("expected user choice"),
        }

        // Empty title → no prompt (options still work).
        let blank_title = json!({
            "toolCall": { "toolCallId": "q", "title": "   " },
            "options": [{ "optionId": "a", "name": "A", "kind": "custom" }]
        });
        match classify_agent_request("session/request_permission", &blank_title) {
            AgentRequestKind::UserChoice { prompt, .. } => {
                assert_eq!(prompt, None);
            }
            other => panic!("expected user choice, got {other:?}"),
        }
    }

    // @spec harness/acp-client Mid-turn user choice: Permission product choice carries prompt from tool title
    #[test]
    fn permission_product_choice_carries_prompt_from_tool_title() {
        // Claude AskUserQuestion bridge: question text is toolCall.title.
        let claude_product = json!({
            "sessionId": "sess-1",
            "toolCall": {
                "toolCallId": "ask-user-1",
                "title": "Ship later or now?"
            },
            "options": [
                { "optionId": "later", "name": "Later", "kind": "custom" },
                { "optionId": "now", "name": "Now", "kind": "custom" }
            ]
        });
        match classify_agent_request("session/request_permission", &claude_product) {
            AgentRequestKind::UserChoice {
                options, prompt, ..
            } => {
                assert_eq!(prompt.as_deref(), Some("Ship later or now?"));
                assert_eq!(options.len(), 2);
                assert_eq!(options[0].label, "Later");
            }
            other => panic!("expected user choice with prompt, got {other:?}"),
        }
    }

    /// Live capture: method `_x.ai/ask_user_question` (and alias) classify as user choice.
    #[test]
    fn classify_live_ask_user_method_name() {
        let params = json!({
            "sessionId": "s",
            "toolCallId": "t",
            "questions": [{
                "question": "Go?",
                "options": [{ "label": "Yes" }, { "label": "No" }]
            }]
        });
        for method in ["_x.ai/ask_user_question", "x.ai/ask_user_question"] {
            match classify_agent_request(method, &params) {
                AgentRequestKind::UserChoice {
                    prompt,
                    options,
                    wire: ChoiceWire::AskUserQuestion { question_text, .. },
                } => {
                    assert_eq!(prompt.as_deref(), Some("Go?"));
                    assert_eq!(question_text, "Go?");
                    assert_eq!(options.len(), 2);
                }
                other => panic!("{method}: expected user choice, got {other:?}"),
            }
        }
        // Unknown method still null path.
        assert!(matches!(
            classify_agent_request("x.ai/other", &params),
            AgentRequestKind::Unknown
        ));
    }

    // @spec harness/acp-client Mid-turn user choice: Host custom freeform answer completes the pending request
    #[test]
    fn host_custom_freeform_answer_completes_the_pending_request() {
        let free = "something else";
        let wire = ChoiceWire::AskUserQuestion {
            question_text: "Ship?".into(),
            options: vec![UserChoiceOption {
                id: "Yes".into(),
                label: "Yes".into(),
            }],
        };
        let result = encode_choice_result(&wire, &UserChoiceAnswer::Custom { text: free.into() });
        assert_eq!(result["outcome"], "accepted", "result={result}");
        assert_eq!(result["answers"]["Ship?"], free);
        assert_ne!(result["outcome"], "skip_interview");

        // Claude parent path uses permission wire; freeform rides as selected optionId.
        let perm = encode_choice_result(
            &ChoiceWire::Permission,
            &UserChoiceAnswer::Custom { text: free.into() },
        );
        assert_eq!(perm["outcome"]["outcome"], "selected");
        assert_eq!(perm["outcome"]["optionId"], free);
    }
}
