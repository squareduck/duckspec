//! ACP agent state: sessions, process-hot app-server, prompt orchestration.
//!
//! `session/new` always opens a Codex thread immediately so the ACP session id
//! is the real `thread.id`. One app-server child stays process-hot across main
//! turns until cancel kills it.

use std::collections::HashSet;
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::codex::ask_user::{
    decode_user_input, decision_from_parent_result, permission_request_params,
};
use crate::codex::{
    AppServer, AppServerError, CodexSpawnFactory, TurnStreamEvent, acp_prompt_to_turn_input,
    default_spawn_factory, map_notification,
};

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

impl From<AppServerError> for AgentError {
    fn from(e: AppServerError) -> Self {
        match e {
            AppServerError::SessionNotFound(m) => AgentError::SessionNotFound(m),
            AppServerError::Spawn(m) | AppServerError::Process(m) | AppServerError::Protocol(m) => {
                AgentError::Process(m)
            }
        }
    }
}

/// Session table + optional process-hot app-server.
pub(crate) struct Agent {
    /// Known ACP session handles (Codex thread ids).
    sessions: HashSet<String>,
    /// Process-hot app-server, if any.
    hot: Option<AppServer>,
    /// Spawn factory for official (or scripted) `codex app-server`.
    factory: CodexSpawnFactory,
    /// In-flight `(thread_id, turn_id)` for best-effort `turn/interrupt` on cancel.
    in_flight: Option<(String, String)>,
}

impl Agent {
    pub(crate) fn new() -> Self {
        Self::with_factory(default_spawn_factory())
    }

    pub(crate) fn with_factory(factory: CodexSpawnFactory) -> Self {
        Self {
            sessions: HashSet::new(),
            hot: None,
            factory,
            in_flight: None,
        }
    }

    /// Test helper: factory that counts app-server spawns.
    #[cfg(test)]
    pub(crate) fn with_counting_factory(
        factory: CodexSpawnFactory,
        counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    ) -> Self {
        Self::with_factory(crate::codex::counting_factory(factory, counter))
    }

    #[cfg(test)]
    pub(crate) fn has_hot_app_server(&mut self) -> bool {
        self.hot.as_mut().is_some_and(|h| h.alive())
    }

    /// ACP `initialize` result: protocol version, loadSession, and models from
    /// live `model/list` (via a short-lived app-server). Empty when discovery
    /// fails so the host does not offer unusable Codex models.
    pub(crate) async fn initialize(&self) -> Value {
        let live = crate::models::discover_live_models(&self.factory).await;
        if let Err(ref e) = live {
            tracing::warn!("codex model live discovery failed, advertising no models: {e}");
        }
        let models = crate::models::resolve_advertised_models(live);
        crate::models::initialize_result(&models)
    }

    /// Open a new session: ensure heat, `thread/start`, return thread id.
    pub(crate) async fn session_new(&mut self, params: &Value) -> Result<Value, AgentError> {
        let cwd = cwd_from_params(params);
        let model = model_from_params(params);

        self.ensure_hot().await?;
        let hot = self.hot.as_mut().expect("ensure_hot leaves hot set");
        let thread_id = hot
            .thread_start(&cwd.to_string_lossy(), model.as_deref())
            .await?;
        self.sessions.insert(thread_id.clone());
        Ok(json!({ "sessionId": thread_id }))
    }

    /// Resume a session: ensure heat, `thread/resume`, missing → session-not-found.
    pub(crate) async fn session_load(&mut self, params: &Value) -> Result<Value, AgentError> {
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| AgentError::InvalidParams("session/load missing sessionId".into()))?
            .to_string();
        let cwd = cwd_from_params(params);
        let _ = cwd; // resume uses thread id; cwd optional override later if needed
        let _ = model_from_params(params);

        self.ensure_hot().await?;
        let hot = self.hot.as_mut().expect("ensure_hot leaves hot set");
        match hot.thread_resume(&session_id).await {
            Ok(id) => {
                self.sessions.insert(id);
                Ok(json!({}))
            }
            Err(e) if e.is_session_not_found() => Err(AgentError::SessionNotFound(e.to_string())),
            Err(e) => Err(e.into()),
        }
    }

    /// Run a prompt on an existing session id (thread id).
    ///
    /// `parent_reader` / `write_acp` bridge mid-turn `item/tool/requestUserInput`
    /// to parent `session/request_permission` (product options).
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
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| AgentError::InvalidParams("session/prompt missing sessionId".into()))?
            .to_string();
        let model = model_from_params(params);
        let mut turn_input = acp_prompt_to_turn_input(params);

        // Ensure heat; if we had to respawn, rejoin the thread before turning.
        let hot_alive = self.hot.as_mut().is_some_and(|h| h.alive());
        let need_resume = !hot_alive || !self.sessions.contains(&session_id);
        self.ensure_hot().await?;
        if need_resume || !self.sessions.contains(&session_id) {
            let hot = self.hot.as_mut().expect("hot after ensure");
            match hot.thread_resume(&session_id).await {
                Ok(id) => {
                    self.sessions.insert(id);
                }
                Err(e) if e.is_session_not_found() => {
                    turn_input.cleanup();
                    return Err(AgentError::SessionNotFound(e.to_string()));
                }
                Err(e) => {
                    turn_input.cleanup();
                    return Err(e.into());
                }
            }
        } else {
            self.sessions.insert(session_id.clone());
        }

        let result = self
            .run_turn_stream(
                &session_id,
                turn_input.blocks.clone(),
                model.as_deref(),
                on_update,
                parent_reader,
                write_acp,
            )
            .await;
        turn_input.cleanup();
        result
    }

    async fn run_turn_stream<R, W>(
        &mut self,
        session_id: &str,
        input: Vec<Value>,
        model: Option<&str>,
        on_update: &mut (dyn FnMut(Value) + Send),
        parent_reader: &mut R,
        write_acp: &mut W,
    ) -> Result<Value, AgentError>
    where
        R: tokio::io::AsyncBufRead + Unpin + Send,
        W: FnMut(Value) -> Result<(), AgentError> + Send,
    {
        let hot = self.hot.as_mut().expect("hot after ensure");
        let turn_id = match hot.turn_start(session_id, input, model).await {
            Ok(id) => id,
            Err(e) => {
                self.in_flight = None;
                return Err(e.into());
            }
        };
        self.in_flight = Some((session_id.to_string(), turn_id.clone()));
        let mut next_parent_id = 10_000u64;

        let outcome = loop {
            match hot.next_stream_event().await {
                Ok(TurnStreamEvent::UserInput(need)) => {
                    let decision = service_parent_choice(
                        &need.params,
                        session_id,
                        parent_reader,
                        write_acp,
                        &mut next_parent_id,
                    )
                    .await;
                    let decision = match decision {
                        Ok(d) => d,
                        Err(e) => break Err(e),
                    };
                    if let Err(e) = hot.complete_user_input(&need.rpc_id, &decision).await {
                        break Err(e.into());
                    }
                }
                Ok(TurnStreamEvent::Notification(ev)) => {
                    for update in map_notification(&ev, session_id) {
                        on_update(update);
                    }
                    if ev.get("method").and_then(Value::as_str) == Some("turn/completed") {
                        let completed_id = ev
                            .pointer("/params/turn/id")
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        if completed_id == turn_id || completed_id.is_empty() {
                            break Ok(json!({ "stopReason": "end_turn" }));
                        }
                    }
                }
                Err(e) => break Err(e.into()),
            }
        };
        self.in_flight = None;
        outcome
    }

    /// Cancel: best-effort `turn/interrupt` when a turn is tracked, then kill
    /// the app-server child (ends process-hot). Host mid-prompt cancel typically
    /// kills this ACP process instead (shared client pattern).
    pub(crate) async fn cancel(&mut self, _session_id: Option<&str>) {
        if let Some(mut hot) = self.hot.take() {
            if let Some((thread_id, turn_id)) = self.in_flight.take() {
                let _ = hot.turn_interrupt(&thread_id, &turn_id).await;
            }
            hot.kill().await;
        }
        self.in_flight = None;
    }

    async fn ensure_hot(&mut self) -> Result<(), AgentError> {
        let dead = match self.hot.as_mut() {
            Some(h) => !h.alive(),
            None => true,
        };
        if dead {
            if let Some(old) = self.hot.take() {
                old.kill().await;
            }
            self.in_flight = None;
            let server = AppServer::connect(&self.factory).await?;
            self.hot = Some(server);
            // Process restart: prior in-memory session membership is invalid
            // until re-resumed/started on the new process.
            self.sessions.clear();
        }
        Ok(())
    }
}

fn cwd_from_params(params: &Value) -> PathBuf {
    params
        .get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn model_from_params(params: &Value) -> Option<String> {
    params
        .get("model")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Issue parent `session/request_permission` for each question and merge answers.
///
/// Any cancel aborts the whole questionnaire.
async fn service_parent_choice<R, W>(
    user_input_params: &Value,
    session_id: &str,
    parent_reader: &mut R,
    write_acp: &mut W,
    next_id: &mut u64,
) -> Result<crate::codex::ask_user::UserInputDecision, AgentError>
where
    R: tokio::io::AsyncBufRead + Unpin + Send,
    W: FnMut(Value) -> Result<(), AgentError> + Send,
{
    use crate::codex::ask_user::{UserInputDecision, merge_answers};
    use tokio::io::AsyncBufReadExt;

    let questions = decode_user_input(user_input_params);
    if questions.is_empty() {
        return Ok(UserInputDecision::Cancelled);
    }
    let tool_call_id = user_input_params
        .get("itemId")
        .and_then(Value::as_str)
        .unwrap_or("user-input");

    let mut parts: Vec<Value> = Vec::with_capacity(questions.len());
    for q in &questions {
        let call_id = format!("{tool_call_id}:{}", q.id);
        let perm_params =
            permission_request_params(session_id, &call_id, q.question.as_deref(), &q.options);

        let req_id = *next_id;
        *next_id += 1;
        let request = json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "method": "session/request_permission",
            "params": perm_params,
        });
        write_acp(request)?;

        // Read parent stdin until we see the response for this request id.
        let mut line = String::new();
        let decision = loop {
            line.clear();
            let n = parent_reader
                .read_line(&mut line)
                .await
                .map_err(|e| AgentError::Process(format!("read parent acp: {e}")))?;
            if n == 0 {
                break UserInputDecision::Cancelled;
            }
            let msg: Value = match serde_json::from_str(line.trim_end()) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let id_match = match msg.get("id") {
                Some(Value::Number(n)) => n.as_u64() == Some(req_id),
                Some(Value::String(s)) => s.parse::<u64>().ok() == Some(req_id),
                _ => false,
            };
            if !id_match {
                // Ignore other parent messages (e.g. cancel) during the wait.
                continue;
            }
            if let Some(result) = msg.get("result") {
                break decision_from_parent_result(result, &q.id, &q.options);
            }
            // Parent error → cancel questionnaire.
            break UserInputDecision::Cancelled;
        };

        match decision {
            UserInputDecision::Cancelled => return Ok(UserInputDecision::Cancelled),
            UserInputDecision::Accepted { answers } => parts.push(answers),
        }
    }

    Ok(UserInputDecision::Accepted {
        answers: merge_answers(parts),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::CodexSpawnFactory;
    use std::process::Stdio;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use tokio::process::Command;

    /// Scripted `codex` peer: argv is ignored; speaks App Server JSON-RPC on stdio.
    const SCRIPTED_CODEX_PY: &str = r#"
import json, sys

def reply(id, result):
    print(json.dumps({"id": id, "result": result}), flush=True)

def err(id, message):
    print(json.dumps({"id": id, "error": {"code": -32600, "message": message}}), flush=True)

threads = set()
turn_n = 0

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        msg = json.loads(line)
    except Exception:
        continue
    method = msg.get("method")
    mid = msg.get("id")
    params = msg.get("params") or {}

    if method == "initialize":
        reply(mid, {
            "userAgent": "fake-codex",
            "codexHome": "/tmp",
            "platformFamily": "unix",
            "platformOs": "macos",
        })
    elif method == "initialized":
        pass
    elif method == "model/list":
        reply(mid, {
            "data": [
                {"id": "gpt-5.4", "displayName": "GPT-5.4", "hidden": False},
                {"id": "gpt-5.4-mini", "displayName": "GPT-5.4-Mini", "hidden": False},
            ],
            "nextCursor": None,
        })
    elif method == "thread/start":
        tid = f"thread-{len(threads)+1}"
        threads.add(tid)
        thread = {
            "id": tid,
            "sessionId": tid,
            "cliVersion": "0",
            "createdAt": 0,
            "updatedAt": 0,
            "cwd": params.get("cwd") or "/tmp",
            "ephemeral": False,
            "modelProvider": "openai",
            "preview": "",
            "source": "test",
            "status": {"type": "idle"},
            "turns": [],
        }
        reply(mid, {
            "thread": thread,
            "model": params.get("model") or "gpt-test",
            "modelProvider": "openai",
            "cwd": thread["cwd"],
            "approvalPolicy": "never",
            "approvalsReviewer": "user",
            "sandbox": {"type": "workspaceWrite"},
        })
        print(json.dumps({"method": "thread/started", "params": {"thread": thread}}), flush=True)
    elif method == "thread/resume":
        tid = params.get("threadId") or ""
        if tid == "missing-session-id" or tid.startswith("missing-"):
            err(mid, f"no rollout found for thread id {tid}")
            continue
        threads.add(tid)
        thread = {
            "id": tid,
            "sessionId": tid,
            "cliVersion": "0",
            "createdAt": 0,
            "updatedAt": 0,
            "cwd": "/tmp",
            "ephemeral": False,
            "modelProvider": "openai",
            "preview": "",
            "source": "test",
            "status": {"type": "idle"},
            "turns": [],
        }
        reply(mid, {
            "thread": thread,
            "model": "gpt-test",
            "modelProvider": "openai",
            "cwd": "/tmp",
            "approvalPolicy": "never",
            "approvalsReviewer": "user",
            "sandbox": {"type": "workspaceWrite"},
        })
    elif method == "turn/start":
        turn_n += 1
        tid = params.get("threadId") or ""
        turn_id = f"turn-{turn_n}"
        turn = {"id": turn_id, "items": [], "status": "inProgress"}
        reply(mid, {"turn": turn})
        print(json.dumps({
            "method": "turn/started",
            "params": {"threadId": tid, "turn": turn},
        }), flush=True)
        # Stream assistant text before completion so profile updates are live.
        print(json.dumps({
            "method": "item/agentMessage/delta",
            "params": {
                "threadId": tid,
                "turnId": turn_id,
                "itemId": "msg-1",
                "delta": "echo-ok",
            },
        }), flush=True)
        print(json.dumps({
            "method": "thread/tokenUsage/updated",
            "params": {
                "threadId": tid,
                "turnId": turn_id,
                "tokenUsage": {
                    "last": {
                        "inputTokens": 1,
                        "cachedInputTokens": 0,
                        "outputTokens": 1,
                        "reasoningOutputTokens": 0,
                        "totalTokens": 2,
                    },
                    "total": {
                        "inputTokens": 1,
                        "cachedInputTokens": 0,
                        "outputTokens": 1,
                        "reasoningOutputTokens": 0,
                        "totalTokens": 2,
                    },
                },
            },
        }), flush=True)
        turn_done = {"id": turn_id, "items": [], "status": "completed"}
        print(json.dumps({
            "method": "turn/completed",
            "params": {"threadId": tid, "turn": turn_done},
        }), flush=True)
    elif method == "turn/interrupt":
        reply(mid, {})
    elif mid is not None:
        err(mid, f"unknown method {method}")
"#;

    fn scripted_factory() -> CodexSpawnFactory {
        Arc::new(|| {
            let mut cmd = Command::new("python3");
            cmd.arg("-u")
                .arg("-c")
                .arg(SCRIPTED_CODEX_PY)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true);
            cmd
        })
    }

    async fn prompt_ok(agent: &mut Agent, session_id: &str, text: &str) -> Value {
        let mut sink = |_u: Value| {};
        let mut parent = tokio::io::BufReader::new(tokio::io::empty());
        let mut write = |_m: Value| -> Result<(), AgentError> { Ok(()) };
        agent
            .run_prompt(
                &json!({
                    "sessionId": session_id,
                    "prompt": [{ "type": "text", "text": text }],
                }),
                &mut sink,
                &mut parent,
                &mut write,
            )
            .await
            .unwrap()
    }

    /// Profile updates are delivered while the turn runs (not only after stop).
    #[tokio::test]
    async fn profile_updates_stream_during_turn() {
        let mut agent = Agent::with_factory(scripted_factory());
        let sid = agent
            .session_new(&json!({ "cwd": std::env::temp_dir().to_string_lossy() }))
            .await
            .unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();

        let saw_before_return = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let returned = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = Arc::clone(&saw_before_return);
        let done = Arc::clone(&returned);
        let mut sink = move |u: Value| {
            if u["update"]["sessionUpdate"] == "agent_message_chunk" {
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
        returned.store(true, AtomicOrdering::SeqCst);
        assert_eq!(result["stopReason"], "end_turn");
        assert!(
            saw_before_return.load(AtomicOrdering::SeqCst),
            "expected live agent_message_chunk during turn"
        );
        agent.cancel(None).await;
    }

    #[tokio::test]
    async fn initialize_advertises_models_from_model_list() {
        let agent = Agent::with_factory(scripted_factory());
        let init = agent.initialize().await;
        let available = init
            .pointer("/_meta/modelState/availableModels")
            .and_then(Value::as_array)
            .expect("availableModels");
        assert!(
            !available.is_empty(),
            "initialize must advertise models, got {init}"
        );
        assert_eq!(available[0]["modelId"], "gpt-5.4");
        assert_eq!(available[0]["name"], "GPT-5.4");
        assert_eq!(available[1]["modelId"], "gpt-5.4-mini");
        // Discovery uses a short-lived process and must not leave heat.
        // (initialize takes &self; heat is only via session_new path)
    }

    /// Agent binary is up; official app-server spawn fails → no models advertised,
    /// and a session open fails with a process error (typed, not panic).
    ///
    /// @spec harness/openai-codex Graceful unavailability: A missing agent or backend yields no models and a turn error
    #[tokio::test]
    async fn backend_down_initialize_advertises_no_models() {
        let factory: CodexSpawnFactory = Arc::new(|| {
            let mut cmd = Command::new("/nonexistent/codex-app-server-does-not-exist");
            cmd.stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true);
            cmd
        });
        let mut agent = Agent::with_factory(factory);
        let init = agent.initialize().await;
        let available = init
            .pointer("/_meta/modelState/availableModels")
            .and_then(Value::as_array)
            .expect("availableModels array present even when empty");
        assert!(
            available.is_empty(),
            "missing backend must not advertise curated models, got {init}"
        );
        let open = agent
            .session_new(&json!({ "cwd": std::env::temp_dir().to_string_lossy() }))
            .await;
        assert!(
            matches!(open, Err(AgentError::Process(_))),
            "missing backend must fail the turn with a typed process error, got {open:?}"
        );
    }

    /// @spec harness/openai-codex Session lifecycle and thread ids: A turn without a prior session opens a new session and surfaces a Codex thread id
    #[tokio::test]
    async fn turn_without_prior_session_surfaces_thread_id() {
        let mut agent = Agent::with_factory(scripted_factory());
        let created = agent
            .session_new(&json!({ "cwd": std::env::temp_dir().to_string_lossy() }))
            .await
            .unwrap();
        let sid = created["sessionId"].as_str().unwrap().to_string();
        assert!(
            sid.starts_with("thread-"),
            "session/new must surface Codex thread id, got {sid}"
        );

        let result = prompt_ok(&mut agent, &sid, "hi").await;
        assert_eq!(result["stopReason"], "end_turn");
        assert!(agent.has_hot_app_server());
        agent.cancel(None).await;
    }

    /// @spec harness/openai-codex Session lifecycle and thread ids: A turn with a prior session id resumes that id
    #[tokio::test]
    async fn turn_with_prior_session_resumes_id() {
        let mut agent = Agent::with_factory(scripted_factory());
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        let created = agent.session_new(&json!({ "cwd": cwd })).await.unwrap();
        let sid = created["sessionId"].as_str().unwrap().to_string();
        prompt_ok(&mut agent, &sid, "first").await;
        agent.cancel(None).await;
        assert!(!agent.has_hot_app_server());

        // Cold load + prompt rejoin the same thread id.
        let mut agent = Agent::with_factory(scripted_factory());
        agent
            .session_load(&json!({
                "sessionId": &sid,
                "cwd": std::env::temp_dir().to_string_lossy(),
            }))
            .await
            .unwrap();
        prompt_ok(&mut agent, &sid, "resume").await;
        agent.cancel(None).await;
    }

    /// @spec harness/openai-codex Session lifecycle and thread ids: A failed load of a missing session surfaces session-not-found
    #[tokio::test]
    async fn failed_load_of_missing_session_is_not_found() {
        let mut agent = Agent::with_factory(scripted_factory());
        let err = agent
            .session_load(&json!({
                "sessionId": "missing-session-id",
                "cwd": std::env::temp_dir().to_string_lossy(),
            }))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AgentError::SessionNotFound(_)),
            "expected SessionNotFound, got {err:?}"
        );
        let rpc = err.to_rpc_value();
        assert_eq!(rpc["data"]["code"], "FS_NOT_FOUND");
        agent.cancel(None).await;
    }

    /// @spec harness/openai-codex App-server process heat: A second main turn reuses the app-server process when hot
    #[tokio::test]
    async fn second_main_turn_reuses_app_server() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut agent = Agent::with_counting_factory(scripted_factory(), Arc::clone(&counter));
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        let sid = agent.session_new(&json!({ "cwd": &cwd })).await.unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1);

        prompt_ok(&mut agent, &sid, "one").await;
        prompt_ok(&mut agent, &sid, "two").await;
        assert_eq!(
            counter.load(AtomicOrdering::SeqCst),
            1,
            "second turn must reuse process-hot app-server"
        );
        agent.cancel(None).await;
    }

    /// @spec harness/openai-codex App-server process heat: After cancel, a later turn may spawn again and resume a prior session id
    #[tokio::test]
    async fn after_cancel_later_turn_may_respawn_and_resume() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut agent = Agent::with_counting_factory(scripted_factory(), Arc::clone(&counter));
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        let sid = agent.session_new(&json!({ "cwd": &cwd })).await.unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();
        prompt_ok(&mut agent, &sid, "before-cancel").await;
        agent.cancel(Some(&sid)).await;
        assert!(!agent.has_hot_app_server());

        prompt_ok(&mut agent, &sid, "after-cancel").await;
        assert!(
            counter.load(AtomicOrdering::SeqCst) >= 2,
            "cancel ends heat; later turn spawns again"
        );
        assert!(agent.has_hot_app_server());
        agent.cancel(None).await;
    }
}
