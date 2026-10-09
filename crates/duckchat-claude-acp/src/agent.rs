//! ACP agent state: sessions, duplex Claude heat, cancel, and profile updates.
//!
//! Official `claude` is not started on `session/new` or cold `session/load`.
//! The first `session/prompt` spawns duplex, writes user content, and binds
//! Claude's native session id (returned on the prompt result when rebound).
//! A later main prompt reuses that process only while it is still that session
//! and was spawned with the prompt's model and effort. Title and reply
//! oneshots are a separate agent process and do not pass `--effort`.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};
use tokio::io::AsyncBufReadExt;
use tokio::sync::{mpsc, oneshot};

use crate::claude::ask_user::{
    self, ASK_USER_QUESTION, ChoiceOption, PermissionDecision, decision_from_cancelled,
    decision_from_selected, decode_ask_user_input, permission_request_params,
};
use crate::claude::duplex::DuplexError;
use crate::claude::{
    ClaudeDuplex, ClaudeSpawnFactory, acp_prompt_to_claude_content, default_spawn_factory,
};

/// Queued AskUserQuestion that needs a parent ACP permission response.
struct ChoiceNeed {
    request_id: String,
    tool_name: String,
    tool_input: Value,
    reply: oneshot::Sender<PermissionDecision>,
}

/// Errors returned from session operations (mapped to JSON-RPC by the loop).
#[derive(Debug)]
pub(crate) enum AgentError {
    SessionNotFound(String),
    InvalidParams(String),
    MethodNotFound(String),
    Process(String),
}

impl AgentError {
    pub(crate) fn to_rpc_value(&self) -> Value {
        match self {
            AgentError::SessionNotFound(detail) => json!({
                "code": -32603,
                "message": "Path not found.",
                "data": {
                    "code": "FS_NOT_FOUND",
                    "detail": detail,
                }
            }),
            AgentError::InvalidParams(message) => json!({
                "code": -32602,
                "message": message,
            }),
            AgentError::MethodNotFound(method) => json!({
                "code": -32601,
                "message": format!("Method not found: {method}"),
            }),
            AgentError::Process(message) => json!({
                "code": -32603,
                "message": message,
            }),
        }
    }
}

impl From<DuplexError> for AgentError {
    fn from(e: DuplexError) -> Self {
        match e {
            DuplexError::SessionNotFound(m) => AgentError::SessionNotFound(m),
            DuplexError::Spawn(m) | DuplexError::Process(m) | DuplexError::Protocol(m) => {
                AgentError::Process(m)
            }
        }
    }
}

/// Open/load state held until the first user prompt starts Claude.
#[derive(Debug, Clone)]
struct PendingOpen {
    cwd: PathBuf,
    model: Option<String>,
    /// `None` = fresh conversation; `Some(id)` = `--resume` that id.
    resume: Option<String>,
}

/// Duplex-hot Claude process plus the model and effort it was spawned with.
struct HotSpawn {
    duplex: ClaudeDuplex,
    model: Option<String>,
    effort: Option<String>,
}

/// Session table + optional duplex-hot Claude process.
pub(crate) struct Agent {
    /// Known ACP session handles (provisional and/or native).
    sessions: HashSet<String>,
    /// Pending open/load state keyed by the ACP id returned to the client.
    pending: HashMap<String, PendingOpen>,
    /// After first bind: provisional open id → Claude native id.
    provisional_to_native: HashMap<String, String>,
    /// Process-hot duplex, if any, with the model and effort of that spawn.
    hot: Option<HotSpawn>,
    /// Spawn factory for the official (or scripted) `claude` CLI.
    factory: ClaudeSpawnFactory,
    /// When true, pass `--permission-mode bypassPermissions`.
    bypass_permissions: bool,
}

impl Agent {
    pub(crate) fn new() -> Self {
        Self::with_factory(default_spawn_factory(), true)
    }

    pub(crate) fn with_factory(factory: ClaudeSpawnFactory, bypass_permissions: bool) -> Self {
        Self {
            sessions: HashSet::new(),
            pending: HashMap::new(),
            provisional_to_native: HashMap::new(),
            hot: None,
            factory,
            bypass_permissions,
        }
    }

    /// Test helper: factory that counts inner Claude spawns.
    #[cfg(test)]
    pub(crate) fn with_counting_factory(
        factory: ClaudeSpawnFactory,
        counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    ) -> Self {
        Self::with_factory(crate::claude::counting_factory(factory, counter), true)
    }

    /// ACP `initialize` result: protocol version, loadSession, the filtered
    /// Claude Code catalog, and that fetch's status. A failed fetch still
    /// returns a result and advertises no models.
    pub(crate) async fn initialize(&self) -> Value {
        crate::models::advertise_initialize().await
    }

    /// Open a new ACP session without starting the official `claude` process.
    /// Returns a provisional session id; native id binds on the first prompt.
    pub(crate) async fn session_new(&mut self, params: &Value) -> Result<Value, AgentError> {
        let cwd = cwd_from_params(params);
        let model = model_from_params(params);

        // Drop any prior heat before opening a new conversation.
        if let Some(hot) = self.hot.take() {
            hot.duplex.kill().await;
        }

        let provisional = new_provisional_id();
        self.sessions.insert(provisional.clone());
        self.pending.insert(
            provisional.clone(),
            PendingOpen {
                cwd,
                model,
                resume: None,
            },
        );
        Ok(json!({ "sessionId": provisional }))
    }

    /// Resume a session id. Reuses duplex-hot process when it already holds
    /// that id (or a provisional that rebound to it); otherwise records cold
    /// load state without spawning Claude.
    pub(crate) async fn session_load(&mut self, params: &Value) -> Result<Value, AgentError> {
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| AgentError::InvalidParams("session/load missing sessionId".into()))?
            .to_string();
        let cwd = cwd_from_params(params);
        let model = model_from_params(params);

        let resolved = self.resolve_id(&session_id);

        if let Some(hot) = self.hot.as_mut() {
            if hot.duplex.session_id == resolved && hot.duplex.alive() {
                self.sessions.insert(session_id);
                return Ok(json!({}));
            }
            // Wrong session or dead process — tear down; cold load records resume.
            if let Some(old) = self.hot.take() {
                old.duplex.kill().await;
            }
        }

        // Cold load: do not spawn Claude until the first prompt.
        self.sessions.insert(session_id.clone());
        self.pending.insert(
            session_id.clone(),
            PendingOpen {
                cwd,
                model,
                resume: Some(resolved),
            },
        );
        Ok(json!({}))
    }

    /// Run a prompt: spawn Claude on first need (write user content then read
    /// init+stream), or reuse duplex heat. Profile updates are delivered live
    /// via `on_update`. Returns a prompt result that includes `sessionId` when
    /// the id rebinds to Claude's native id.
    ///
    /// `parent_reader` is the ACP parent's stdin — read mid-prompt for answers
    /// to agent→parent `session/request_permission`. `write_acp` emits all
    /// parent-facing JSON-RPC lines (updates + requests + results elsewhere).
    pub(crate) async fn run_prompt<R, W>(
        &mut self,
        params: &Value,
        on_update: &mut (dyn FnMut(Value) + Send),
        parent_reader: &mut R,
        write_acp: &mut W,
    ) -> Result<Value, AgentError>
    where
        R: tokio::io::AsyncBufRead + Unpin + Send,
        W: FnMut(Value) -> Result<(), AgentError> + Send,
    {
        let request_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| AgentError::InvalidParams("session/prompt missing sessionId".into()))?
            .to_string();
        let content = acp_prompt_to_claude_content(params);

        let open_id = request_id.clone();
        self.ensure_hot_and_prompt(
            &request_id,
            params,
            content,
            on_update,
            parent_reader,
            write_acp,
        )
        .await?;

        let live_id = self
            .hot
            .as_ref()
            .map(|h| h.duplex.session_id.clone())
            .unwrap_or_else(|| request_id.clone());
        self.sessions.insert(live_id.clone());

        let mut result = json!({ "stopReason": "end_turn" });
        if live_id != open_id {
            result["sessionId"] = json!(live_id);
        }

        Ok(result)
    }

    async fn ensure_hot_and_prompt<R, W>(
        &mut self,
        request_id: &str,
        params: &Value,
        content: Vec<Value>,
        on_update: &mut (dyn FnMut(Value) + Send),
        parent_reader: &mut R,
        write_acp: &mut W,
    ) -> Result<(), AgentError>
    where
        R: tokio::io::AsyncBufRead + Unpin + Send,
        W: FnMut(Value) -> Result<(), AgentError> + Send,
    {
        let resolved = self.resolve_id(request_id);
        let prompt_model = model_from_params(params);
        let prompt_effort = effort_from_params(params);

        // Reuse only while this session is alive and was spawned with this
        // prompt's model and effort. A mismatch takes the cold path, which
        // resumes the same native id. Model and effort are read from the
        // prompt that is starting, so a picker change during an in-flight
        // turn does not cancel that turn.
        let drop_hot = match self.hot.as_mut() {
            Some(hot) => {
                hot.duplex.session_id != resolved
                    || !hot.duplex.alive()
                    || hot.model != prompt_model
                    || hot.effort != prompt_effort
            }
            None => false,
        };
        if drop_hot && let Some(old) = self.hot.take() {
            old.duplex.kill().await;
        }

        if self.hot.is_some() {
            // Reuse process-hot duplex.
            self.prompt_with_parent_bridge(content, on_update, parent_reader, write_acp, &resolved)
                .await?;
            return Ok(());
        }

        // First prompt for this handle: spawn with first user content.
        let pending = self.pending.remove(request_id).or_else(|| {
            // Client may only know a persisted native id (no prior open on this process).
            Some(PendingOpen {
                cwd: cwd_from_params(params),
                model: model_from_params(params),
                resume: Some(resolved.clone()),
            })
        });

        let pending = pending.expect("pending open always Some");
        // Prefer params cwd/model when present; else pending from open/load.
        let cwd = {
            let from_params = params.get("cwd").and_then(Value::as_str);
            if from_params.is_some() {
                cwd_from_params(params)
            } else {
                pending.cwd
            }
        };
        let model = model_from_params(params).or(pending.model);
        let effort = effort_from_params(params);
        // Kept aside: the open future borrows `model` and `effort` until it
        // drops at the end of this function.
        let spawned_model = model.clone();
        let spawned_effort = effort.clone();
        let resume = pending.resume.clone();
        let factory = self.factory.clone();
        let bypass = self.bypass_permissions;
        let session_for_choice = resume.clone().unwrap_or_else(|| resolved.clone());

        let (need_tx, mut need_rx) = mpsc::unbounded_channel::<ChoiceNeed>();
        let mut resolve = make_channel_resolver(need_tx);
        let mut next_req_id = 10_000u64;

        let open_fut = ClaudeDuplex::open_with_first_prompt_resolved(
            &factory,
            &cwd,
            resume.as_deref(),
            model.as_deref(),
            effort.as_deref(),
            bypass,
            content,
            on_update,
            &mut resolve,
        );
        tokio::pin!(open_fut);
        let duplex = loop {
            tokio::select! {
                biased;
                Some(need) = need_rx.recv() => {
                    let decision = service_parent_choice(
                        &need.request_id,
                        &need.tool_name,
                        &need.tool_input,
                        parent_reader,
                        write_acp,
                        &session_for_choice,
                        &mut next_req_id,
                    ).await?;
                    let _ = need.reply.send(decision);
                }
                result = &mut open_fut => {
                    break result?;
                }
            }
        };

        let native = duplex.session_id.clone();
        if native != request_id {
            self.provisional_to_native
                .insert(request_id.to_string(), native.clone());
        }
        self.sessions.insert(native.clone());
        self.hot = Some(HotSpawn {
            duplex,
            model: spawned_model,
            effort: spawned_effort,
        });
        Ok(())
    }

    /// Prompt the process-hot duplex with parent choice bridging.
    async fn prompt_with_parent_bridge<R, W>(
        &mut self,
        content: Vec<Value>,
        on_update: &mut (dyn FnMut(Value) + Send),
        parent_reader: &mut R,
        write_acp: &mut W,
        session_id: &str,
    ) -> Result<(), AgentError>
    where
        R: tokio::io::AsyncBufRead + Unpin + Send,
        W: FnMut(Value) -> Result<(), AgentError> + Send,
    {
        let mut hot = self
            .hot
            .take()
            .ok_or_else(|| AgentError::Process("no hot claude".into()))?;

        let (need_tx, mut need_rx) = mpsc::unbounded_channel::<ChoiceNeed>();
        let mut resolve = make_channel_resolver(need_tx);
        let mut next_req_id = 10_000u64;
        let session_id = session_id.to_string();

        let result = {
            let prompt_fut = hot
                .duplex
                .prompt_with_resolver(content, on_update, &mut resolve);
            tokio::pin!(prompt_fut);

            loop {
                tokio::select! {
                    biased;
                    Some(need) = need_rx.recv() => {
                        let decision = service_parent_choice(
                            &need.request_id,
                            &need.tool_name,
                            &need.tool_input,
                            parent_reader,
                            write_acp,
                            &session_id,
                            &mut next_req_id,
                        ).await?;
                        let _ = need.reply.send(decision);
                    }
                    result = &mut prompt_fut => {
                        break result;
                    }
                }
            }
        };

        self.hot = Some(hot);
        result.map_err(AgentError::from)
    }

    /// Resolve provisional open ids to the native id they rebound to.
    fn resolve_id(&self, id: &str) -> String {
        self.provisional_to_native
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_string())
    }

    /// Cancel tears down duplex heat. Session ids stay known so a later turn
    /// may re-spawn and resume.
    pub(crate) async fn cancel(&mut self, _session_id: Option<&str>) {
        if let Some(hot) = self.hot.take() {
            hot.duplex.kill().await;
        }
    }

    /// Test/observability: whether an official Claude process is currently hot.
    #[cfg(test)]
    pub(crate) fn has_hot_claude(&self) -> bool {
        self.hot.is_some()
    }
}

fn new_provisional_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("pending-{n}")
}

fn cwd_from_params(params: &Value) -> PathBuf {
    params
        .get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn model_from_params(params: &Value) -> Option<String> {
    // ACP clients may put model under `_meta` or top-level; accept both.
    params
        .pointer("/_meta/model")
        .or_else(|| params.get("model"))
        .and_then(Value::as_str)
        .filter(|model| !model.is_empty())
        .map(str::to_string)
}

/// Effort level copied onto `session/prompt` as `effort`. Absent or empty
/// means this prompt has no level, so the spawn omits `--effort`.
fn effort_from_params(params: &Value) -> Option<String> {
    params
        .get("effort")
        .and_then(Value::as_str)
        .filter(|level| !level.is_empty())
        .map(str::to_string)
}

type ChoiceResolveFuture = std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<PermissionDecision, DuplexError>> + Send>,
>;

/// Resolver that posts AskUserQuestion needs onto a channel (no stack borrows).
fn make_channel_resolver(
    tx: mpsc::UnboundedSender<ChoiceNeed>,
) -> impl FnMut(String, String, Value) -> ChoiceResolveFuture {
    move |request_id: String, tool_name: String, tool_input: Value| {
        let tx = tx.clone();
        Box::pin(async move {
            if tool_name != ASK_USER_QUESTION {
                return Ok(ask_user::auto_allow_ordinary_tool());
            }
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(ChoiceNeed {
                request_id,
                tool_name,
                tool_input,
                reply: reply_tx,
            })
            .map_err(|_| DuplexError::Process("parent choice channel closed".into()))?;
            reply_rx
                .await
                .map_err(|_| DuplexError::Process("parent choice dropped".into()))
        })
    }
}

/// Issue `session/request_permission` to the parent and wait for its JSON-RPC result.
async fn service_parent_choice<R, W>(
    control_request_id: &str,
    tool_name: &str,
    tool_input: &Value,
    parent_reader: &mut R,
    write_acp: &mut W,
    session_id: &str,
    next_req_id: &mut u64,
) -> Result<PermissionDecision, AgentError>
where
    R: tokio::io::AsyncBufRead + Unpin + Send,
    W: FnMut(Value) -> Result<(), AgentError> + Send,
{
    if tool_name != ASK_USER_QUESTION {
        return Ok(ask_user::auto_allow_ordinary_tool());
    }
    let (question, options) = decode_ask_user_input(tool_input);
    if options.is_empty() {
        return Ok(decision_from_cancelled());
    }
    let rpc_id = {
        let id = *next_req_id;
        *next_req_id += 1;
        id
    };
    let tool_call_id = format!("ask-user-{control_request_id}");
    let params =
        permission_request_params(session_id, &tool_call_id, question.as_deref(), &options);
    write_acp(json!({
        "jsonrpc": "2.0",
        "id": rpc_id,
        "method": "session/request_permission",
        "params": params,
    }))?;

    let mut line = String::new();
    loop {
        line.clear();
        let n = parent_reader
            .read_line(&mut line)
            .await
            .map_err(|e| AgentError::Process(format!("read parent: {e}")))?;
        if n == 0 {
            return Err(AgentError::Process(
                "parent closed stdin while awaiting user choice".into(),
            ));
        }
        let msg: Value = match serde_json::from_str(line.trim_end()) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if msg.get("method").is_some() {
            continue;
        }
        if msg.get("id") != Some(&json!(rpc_id)) {
            continue;
        }
        return Ok(map_parent_permission_result(
            &msg,
            tool_input,
            question.as_deref().unwrap_or(""),
            &options,
        ));
    }
}

/// Map a parent `session/request_permission` JSON-RPC result to a Claude decision.
fn map_parent_permission_result(
    msg: &Value,
    tool_input: &Value,
    question_text: &str,
    options: &[ChoiceOption],
) -> PermissionDecision {
    let outcome = msg.pointer("/result/outcome");
    let kind = outcome
        .and_then(|o| o.get("outcome"))
        .and_then(Value::as_str)
        .unwrap_or("cancelled");
    match kind {
        "selected" => {
            let option_id = outcome
                .and_then(|o| o.get("optionId"))
                .and_then(Value::as_str)
                .unwrap_or("");
            decision_from_selected(tool_input, question_text, option_id, options)
        }
        _ => decision_from_cancelled(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::duplex::ClaudeSpawnArgs;
    use std::process::Stdio;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use tokio::process::Command;

    /// Scripted Claude peer for `python3 -c` (stdin is the duplex pipe).
    const SCRIPTED_CLAUDE_PY: &str = r#"
import json, sys, os
resume = os.environ.get("RESUME") or None
if resume == "":
    resume = None
if resume == "missing-session-id":
    print(json.dumps({
        "type": "result",
        "is_error": True,
        "result": "No conversation found with session ID: missing-session-id",
    }), flush=True)
    sys.exit(1)
session_id = resume or "claude-native-sess-1"
print(json.dumps({
    "type": "system",
    "subtype": "init",
    "session_id": session_id,
    "model": "sonnet",
}), flush=True)
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        msg = json.loads(line)
    except Exception:
        continue
    text = ""
    if msg.get("type") == "user":
        content = (msg.get("message") or {}).get("content") or []
        for b in content:
            if b.get("type") == "text":
                text += b.get("text") or ""
    if text.startswith("TOOL:"):
        print(json.dumps({
            "type": "assistant",
            "message": {
                "content": [{
                    "type": "tool_use",
                    "id": "call-1",
                    "name": "Read",
                    "input": {"path": "x.rs"}
                }]
            }
        }), flush=True)
        print(json.dumps({
            "type": "user",
            "message": {
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": "call-1",
                    "content": "ok-bytes"
                }]
            }
        }), flush=True)
    print(json.dumps({
        "type": "stream_event",
        "event": {
            "type": "content_block_delta",
            "delta": {"type": "text_delta", "text": "echo:" + text},
        },
    }), flush=True)
    print(json.dumps({
        "type": "result",
        "subtype": "success",
        "is_error": False,
        "session_id": session_id,
        "result": "ok",
    }), flush=True)
"#;

    fn scripted_command(args: &ClaudeSpawnArgs) -> Command {
        let mut cmd = Command::new("python3");
        cmd.arg("-u")
            .arg("-c")
            .arg(SCRIPTED_CLAUDE_PY)
            .env("RESUME", args.resume.clone().unwrap_or_default())
            .current_dir(&args.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        cmd
    }

    fn scripted_factory() -> ClaudeSpawnFactory {
        Arc::new(scripted_command)
    }

    /// One production `claude` argv plus the spawn args the agent handed over.
    #[derive(Clone, Debug)]
    struct SeenSpawn {
        resume: Option<String>,
        model: Option<String>,
        effort: Option<String>,
        argv: Vec<String>,
    }

    fn remember_spawn(log: &Mutex<Vec<SeenSpawn>>, args: &ClaudeSpawnArgs) {
        let real = crate::claude::default_spawn_factory();
        let built = real(args);
        let argv = built
            .as_std()
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        log.lock().unwrap().push(SeenSpawn {
            resume: args.resume.clone(),
            model: args.model.clone(),
            effort: args.effort.clone(),
            argv,
        });
    }

    /// Records the command the production factory would run, then spawns the
    /// scripted peer so the turn can finish without a real `claude` binary.
    fn recording_factory(log: Arc<Mutex<Vec<SeenSpawn>>>) -> ClaudeSpawnFactory {
        Arc::new(move |args: &ClaudeSpawnArgs| {
            remember_spawn(&log, args);
            scripted_command(args)
        })
    }

    fn flag_value<'a>(argv: &'a [String], flag: &str) -> Option<&'a str> {
        argv.windows(2)
            .find(|w| w[0] == flag)
            .map(|w| w[1].as_str())
    }

    /// @spec harness/claude Session lifecycle and native session ids: Opening a new session does not start the official claude process before the first user prompt
    #[tokio::test]
    async fn opening_new_session_does_not_start_claude_before_first_prompt() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut agent = Agent::with_counting_factory(scripted_factory(), Arc::clone(&counter));
        let created = agent
            .session_new(&json!({ "cwd": std::env::temp_dir().to_string_lossy() }))
            .await
            .unwrap();
        let sid = created["sessionId"].as_str().unwrap();
        assert!(
            sid.starts_with("pending-"),
            "open returns a provisional handle, got {sid}"
        );
        assert_eq!(
            counter.load(AtomicOrdering::SeqCst),
            0,
            "session/new must not spawn official claude"
        );
        assert!(!agent.has_hot_claude());
        agent.cancel(None).await;
    }

    async fn prompt_collecting(
        agent: &mut Agent,
        params: Value,
    ) -> Result<(Vec<Value>, Value), AgentError> {
        let mut updates = Vec::new();
        let mut sink = |u: Value| updates.push(u);
        let mut parent = tokio::io::BufReader::new(tokio::io::empty());
        let mut write = |_m: Value| -> Result<(), AgentError> { Ok(()) };
        let result = agent
            .run_prompt(&params, &mut sink, &mut parent, &mut write)
            .await?;
        Ok((updates, result))
    }

    /// @spec harness/claude Session lifecycle and native session ids: A turn without a prior session opens a new session and surfaces Claude's native session id
    #[tokio::test]
    async fn turn_without_prior_session_surfaces_native_id() {
        let mut agent = Agent::with_factory(scripted_factory(), true);
        let created = agent
            .session_new(&json!({ "cwd": std::env::temp_dir().to_string_lossy() }))
            .await
            .unwrap();
        let provisional = created["sessionId"].as_str().unwrap().to_string();
        assert!(provisional.starts_with("pending-"));

        let (updates, result) = prompt_collecting(
            &mut agent,
            json!({
                "sessionId": provisional,
                "prompt": [{ "type": "text", "text": "hi" }],
            }),
        )
        .await
        .unwrap();
        assert_eq!(result["stopReason"], "end_turn");
        assert_eq!(
            result["sessionId"].as_str(),
            Some("claude-native-sess-1"),
            "first prompt rebinds to Claude native id"
        );
        assert_eq!(
            agent.hot.as_ref().map(|h| h.duplex.session_id.as_str()),
            Some("claude-native-sess-1")
        );
        assert!(
            updates
                .iter()
                .any(|u| u["update"]["sessionUpdate"] == "agent_message_chunk")
        );
        agent.cancel(None).await;
    }

    /// @spec harness/claude Session lifecycle and native session ids: A turn with a prior Claude session id resumes that id
    #[tokio::test]
    async fn turn_with_prior_session_resumes_id() {
        let mut agent = Agent::with_factory(scripted_factory(), true);
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        let created = agent.session_new(&json!({ "cwd": cwd })).await.unwrap();
        let provisional = created["sessionId"].as_str().unwrap().to_string();
        let (_, result) = prompt_collecting(
            &mut agent,
            json!({
                "sessionId": provisional,
                "prompt": [{ "type": "text", "text": "first" }],
            }),
        )
        .await
        .unwrap();
        let native = result["sessionId"]
            .as_str()
            .unwrap_or(&provisional)
            .to_string();
        assert_eq!(native, "claude-native-sess-1");
        agent.cancel(None).await; // end heat

        // Cold load records resume without spawning.
        let counter = Arc::new(AtomicUsize::new(0));
        let mut agent = Agent::with_counting_factory(scripted_factory(), Arc::clone(&counter));
        agent
            .session_load(&json!({
                "sessionId": native,
                "cwd": std::env::temp_dir().to_string_lossy(),
            }))
            .await
            .unwrap();
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 0);
        assert!(!agent.has_hot_claude());

        prompt_collecting(
            &mut agent,
            json!({
                "sessionId": native,
                "prompt": [{ "type": "text", "text": "resume" }],
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            agent.hot.as_ref().map(|h| h.duplex.session_id.as_str()),
            Some(native.as_str())
        );
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1);
        agent.cancel(None).await;
    }

    /// @spec harness/claude Duplex main heat: Hot process follows the prompt's model and effort
    #[tokio::test]
    async fn second_main_turn_reuses_inner_claude() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut agent = Agent::with_counting_factory(scripted_factory(), Arc::clone(&counter));
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        let sid = agent.session_new(&json!({ "cwd": &cwd })).await.unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 0);

        let (_, result) = prompt_collecting(
            &mut agent,
            json!({
                "sessionId": sid,
                "prompt": [{ "type": "text", "text": "one" }],
            }),
        )
        .await
        .unwrap();
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1);
        let live = result["sessionId"]
            .as_str()
            .unwrap_or(sid.as_str())
            .to_string();

        prompt_collecting(
            &mut agent,
            json!({
                "sessionId": live,
                "prompt": [{ "type": "text", "text": "two" }],
            }),
        )
        .await
        .unwrap();
        // Provisional id also still routes to hot after rebind.
        prompt_collecting(
            &mut agent,
            json!({
                "sessionId": sid,
                "prompt": [{ "type": "text", "text": "three" }],
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            counter.load(AtomicOrdering::SeqCst),
            1,
            "second turn must reuse duplex-hot claude"
        );
        agent.cancel(None).await;
    }

    /// @spec harness/claude Duplex main heat: After cancel, a later turn may start Claude again and resume a prior session id
    #[tokio::test]
    async fn after_cancel_later_turn_may_respawn_and_resume() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut agent = Agent::with_counting_factory(scripted_factory(), Arc::clone(&counter));
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        let sid = agent.session_new(&json!({ "cwd": &cwd })).await.unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();
        let (_, result) = prompt_collecting(
            &mut agent,
            json!({
                "sessionId": sid,
                "prompt": [{ "type": "text", "text": "before-cancel" }],
            }),
        )
        .await
        .unwrap();
        let native = result["sessionId"]
            .as_str()
            .unwrap_or(sid.as_str())
            .to_string();
        agent.cancel(Some(&native)).await;
        assert!(agent.hot.is_none());

        prompt_collecting(
            &mut agent,
            json!({
                "sessionId": native,
                "prompt": [{ "type": "text", "text": "after-cancel" }],
            }),
        )
        .await
        .unwrap();
        assert!(
            counter.load(AtomicOrdering::SeqCst) >= 2,
            "cancel ends heat; later turn spawns again"
        );
        assert_eq!(
            agent.hot.as_ref().map(|h| h.duplex.session_id.as_str()),
            Some(native.as_str())
        );
        agent.cancel(None).await;
    }

    #[tokio::test]
    async fn missing_session_prompt_is_not_found() {
        let mut agent = Agent::with_factory(scripted_factory(), true);
        // Cold load does not spawn; miss is detected on first prompt resume.
        agent
            .session_load(&json!({
                "sessionId": "missing-session-id",
                "cwd": std::env::temp_dir().to_string_lossy(),
            }))
            .await
            .unwrap();
        let mut sink = |_u: Value| {};
        let mut parent = tokio::io::BufReader::new(tokio::io::empty());
        let mut write = |_m: Value| -> Result<(), AgentError> { Ok(()) };
        let err = agent
            .run_prompt(
                &json!({
                    "sessionId": "missing-session-id",
                    "prompt": [{ "type": "text", "text": "hi" }],
                }),
                &mut sink,
                &mut parent,
                &mut write,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AgentError::SessionNotFound(_)));
    }

    /// @spec harness/claude Profile-compatible event emission: Profile updates are delivered to the host before the turn completes
    #[tokio::test]
    async fn profile_updates_delivered_before_turn_completes() {
        // Scripted peer that emits text before result; sink must see the update
        // while run_prompt is still running (before it returns the result).
        let mut agent = Agent::with_factory(scripted_factory(), true);
        let created = agent
            .session_new(&json!({ "cwd": std::env::temp_dir().to_string_lossy() }))
            .await
            .unwrap();
        let sid = created["sessionId"].as_str().unwrap().to_string();

        let saw_update_before_return = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let result_returned = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = Arc::clone(&saw_update_before_return);
        let done = Arc::clone(&result_returned);
        let mut sink = move |u: Value| {
            if u["update"]["sessionUpdate"] == "agent_message_chunk" {
                // run_prompt has not returned yet if this fires first.
                assert!(
                    !done.load(AtomicOrdering::SeqCst),
                    "session/update must arrive before prompt result"
                );
                flag.store(true, AtomicOrdering::SeqCst);
            }
        };

        let mut parent = tokio::io::BufReader::new(tokio::io::empty());
        let mut write = |_m: Value| -> Result<(), AgentError> { Ok(()) };
        let result = agent
            .run_prompt(
                &json!({
                    "sessionId": sid,
                    "prompt": [{ "type": "text", "text": "stream-me" }],
                }),
                &mut sink,
                &mut parent,
                &mut write,
            )
            .await
            .unwrap();
        result_returned.store(true, AtomicOrdering::SeqCst);

        assert_eq!(result["stopReason"], "end_turn");
        assert!(
            saw_update_before_return.load(AtomicOrdering::SeqCst),
            "host must receive a profile update before the turn's prompt result"
        );
        agent.cancel(None).await;
    }

    fn prompt_with_model(session_id: &str, text: &str, model: &str, effort: Option<&str>) -> Value {
        let mut params = json!({
            "sessionId": session_id,
            "prompt": [{ "type": "text", "text": text }],
            "model": model,
        });
        if let Some(level) = effort {
            params["effort"] = json!(level);
        }
        params
    }

    /// @spec harness/claude Duplex main heat: Hot process follows the prompt's model and effort
    #[tokio::test]
    async fn hot_process_follows_the_prompts_model_and_effort() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut agent = Agent::with_factory(recording_factory(Arc::clone(&log)), true);
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        let sid = agent.session_new(&json!({ "cwd": &cwd })).await.unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();

        // GIVEN a duplex-hot process spawned with a model and an effort level.
        let (_, opened) = prompt_collecting(
            &mut agent,
            prompt_with_model(&sid, "open", "claude-sonnet-5-5", Some("medium")),
        )
        .await
        .unwrap();
        let native = opened["sessionId"].as_str().unwrap().to_string();
        assert_eq!(native, "claude-native-sess-1");
        {
            let spawns = log.lock().unwrap();
            assert_eq!(spawns.len(), 1);
            assert!(spawns[0].resume.is_none());
            assert_eq!(
                flag_value(&spawns[0].argv, "--model"),
                Some("claude-sonnet-5-5")
            );
            assert_eq!(flag_value(&spawns[0].argv, "--effort"), Some("medium"));
        }

        // WHEN a later main prompt matches that spawn.
        prompt_collecting(
            &mut agent,
            prompt_with_model(&native, "match", "claude-sonnet-5-5", Some("medium")),
        )
        .await
        .unwrap();
        assert_eq!(
            log.lock().unwrap().len(),
            1,
            "matching prompt reuses the hot process"
        );
        assert_eq!(
            agent.hot.as_ref().map(|hot| hot.duplex.session_id.as_str()),
            Some(native.as_str())
        );

        // WHEN another prompt changes the model, another changes the effort,
        // and another has no effort level.
        let cases = [
            ("claude-opus", Some("medium")),
            ("claude-opus", Some("high")),
            ("claude-opus", None),
        ];
        for (model, effort) in cases {
            prompt_collecting(
                &mut agent,
                prompt_with_model(&native, "next", model, effort),
            )
            .await
            .unwrap();
            let spawns = log.lock().unwrap();
            let seen = spawns.last().unwrap();
            assert_eq!(seen.resume.as_deref(), Some(native.as_str()));
            assert_eq!(seen.model.as_deref(), Some(model));
            assert_eq!(seen.effort.as_deref(), effort);
            assert_eq!(flag_value(&seen.argv, "--resume"), Some(native.as_str()));
            assert_eq!(flag_value(&seen.argv, "--model"), Some(model));
            assert_eq!(flag_value(&seen.argv, "--effort"), effort);
            assert_eq!(
                agent.hot.as_ref().map(|hot| hot.duplex.session_id.as_str()),
                Some(native.as_str()),
                "cold resume keeps the same native session id"
            );
        }
        assert_eq!(log.lock().unwrap().len(), 4);
        agent.cancel(None).await;
    }

    /// Scripted peer that emits one chunk, then waits for `RELEASE_FILE`
    /// before the result. `INFLIGHT_FILE` is created once the turn is in flight.
    const HOLDING_CLAUDE_PY: &str = r#"
import json, sys, os, time
resume = os.environ.get("RESUME") or None
if resume == "":
    resume = None
session_id = resume or "claude-native-sess-1"
inflight = os.environ["INFLIGHT_FILE"]
release = os.environ["RELEASE_FILE"]
print(json.dumps({
    "type": "system",
    "subtype": "init",
    "session_id": session_id,
    "model": "sonnet",
}), flush=True)
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    print(json.dumps({
        "type": "stream_event",
        "event": {
            "type": "content_block_delta",
            "delta": {"type": "text_delta", "text": "partial"},
        },
    }), flush=True)
    open(inflight, "w").close()
    while not os.path.exists(release):
        time.sleep(0.02)
    print(json.dumps({
        "type": "result",
        "subtype": "success",
        "is_error": False,
        "session_id": session_id,
        "result": "ok",
    }), flush=True)
    break
"#;

    fn holding_factory(
        log: Arc<Mutex<Vec<SeenSpawn>>>,
        inflight: std::path::PathBuf,
        release: std::path::PathBuf,
    ) -> ClaudeSpawnFactory {
        Arc::new(move |args: &ClaudeSpawnArgs| {
            remember_spawn(&log, args);
            let mut cmd = Command::new("python3");
            cmd.arg("-u")
                .arg("-c")
                .arg(HOLDING_CLAUDE_PY)
                .env("RESUME", args.resume.clone().unwrap_or_default())
                .env("INFLIGHT_FILE", &inflight)
                .env("RELEASE_FILE", &release)
                .current_dir(&args.cwd)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true);
            cmd
        })
    }

    /// @spec harness/claude Duplex main heat: A model or effort change during an in-flight turn does not cancel that turn
    #[tokio::test]
    async fn model_or_effort_change_during_in_flight_turn_does_not_cancel() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let dir = std::env::temp_dir().join(format!(
            "duckchat-claude-inflight-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let inflight = dir.join("inflight");
        let release = dir.join("release");
        let mut agent = Agent::with_factory(
            holding_factory(Arc::clone(&log), inflight.clone(), release.clone()),
            true,
        );
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        let sid = agent.session_new(&json!({ "cwd": &cwd })).await.unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();

        // GIVEN an in-flight Claude main turn.
        let params = prompt_with_model(&sid, "running", "claude-sonnet-5-5", Some("medium"));
        let running = tokio::spawn(async move {
            let outcome = prompt_collecting(&mut agent, params).await;
            (outcome, agent)
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !inflight.exists() {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("turn should still be in flight");
        assert_eq!(log.lock().unwrap().len(), 1);

        // WHEN the chat's model or effort changes. The picker is host state
        // for the following prompt; this turn is not cancelled.
        let changed_model = "claude-opus";
        let changed_effort = "max";
        assert_eq!(
            log.lock().unwrap().len(),
            1,
            "changing the picker must not spawn or kill during the turn"
        );

        std::fs::write(&release, b"go").unwrap();
        let (outcome, mut agent) = tokio::time::timeout(std::time::Duration::from_secs(5), running)
            .await
            .expect("in-flight turn should finish")
            .unwrap();
        let (_updates, result) = outcome.unwrap();
        assert_eq!(result["stopReason"], "end_turn");
        assert_eq!(log.lock().unwrap().len(), 1);
        let hot = agent.hot.as_ref().unwrap();
        assert_eq!(hot.model.as_deref(), Some("claude-sonnet-5-5"));
        assert_eq!(hot.effort.as_deref(), Some("medium"));
        let native = hot.duplex.session_id.clone();

        // The new model and effort apply on the following prompt.
        prompt_collecting(
            &mut agent,
            prompt_with_model(&native, "later", changed_model, Some(changed_effort)),
        )
        .await
        .unwrap();
        assert_eq!(log.lock().unwrap().len(), 2);
        assert_eq!(
            agent.hot.as_ref().unwrap().model.as_deref(),
            Some(changed_model)
        );
        assert_eq!(
            agent.hot.as_ref().unwrap().effort.as_deref(),
            Some(changed_effort)
        );
        agent.cancel(None).await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// @spec harness/claude Duplex main heat: A title or reply oneshot leaves the main hot process in place
    #[tokio::test]
    async fn title_or_reply_oneshot_leaves_the_main_hot_process_in_place() {
        let main_log = Arc::new(Mutex::new(Vec::new()));
        let mut main = Agent::with_factory(recording_factory(Arc::clone(&main_log)), true);
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        let sid = main.session_new(&json!({ "cwd": &cwd })).await.unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();
        // GIVEN a duplex-hot Claude main process.
        prompt_collecting(
            &mut main,
            prompt_with_model(&sid, "main", "claude-sonnet-5-5", Some("medium")),
        )
        .await
        .unwrap();
        let native = main.hot.as_ref().unwrap().duplex.session_id.clone();
        assert_eq!(main_log.lock().unwrap().len(), 1);

        // WHEN a title-summary or reply-suggestion oneshot runs. Those sends
        // are a separate agent process and omit effort (AcpTurn::prompt).
        let oneshot_log = Arc::new(Mutex::new(Vec::new()));
        let mut oneshot = Agent::with_factory(recording_factory(Arc::clone(&oneshot_log)), true);
        let oneshot_sid = oneshot.session_new(&json!({ "cwd": &cwd })).await.unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();
        let mut live = oneshot_sid;
        for text in ["title summary of the opening message", "reply suggestion"] {
            let (_, result) = prompt_collecting(
                &mut oneshot,
                prompt_with_model(&live, text, "claude-haiku-4-5", None),
            )
            .await
            .unwrap();
            if let Some(rebound) = result["sessionId"].as_str() {
                live = rebound.to_string();
            }
        }

        // THEN the main hot process is still in place, and the oneshot spawn
        // does not pass an effort flag.
        assert!(main.has_hot_claude());
        assert_eq!(main.hot.as_ref().unwrap().duplex.session_id, native);
        assert_eq!(main_log.lock().unwrap().len(), 1);
        {
            let oneshots = oneshot_log.lock().unwrap();
            assert_eq!(
                oneshots.len(),
                1,
                "the reply oneshot reuses its own process"
            );
            assert!(oneshots[0].effort.is_none());
            assert!(flag_value(&oneshots[0].argv, "--effort").is_none());
            assert_eq!(
                flag_value(&oneshots[0].argv, "--model"),
                Some("claude-haiku-4-5")
            );
        }
        main.cancel(None).await;
        oneshot.cancel(None).await;
    }
}
