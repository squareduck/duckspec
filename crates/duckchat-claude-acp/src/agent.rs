//! ACP agent state: sessions, duplex Claude heat, cancel, and profile updates.
//!
//! Official `claude` is not started on `session/new` or cold `session/load`.
//! The first `session/prompt` spawns duplex, writes user content, and binds
//! Claude's native session id (returned on the prompt result when rebound).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

use crate::claude::duplex::DuplexError;
use crate::claude::{
    ClaudeDuplex, ClaudeSpawnFactory, acp_prompt_to_claude_content, default_spawn_factory,
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

/// Session table + optional duplex-hot Claude process.
pub(crate) struct Agent {
    /// Known ACP session handles (provisional and/or native).
    sessions: HashSet<String>,
    /// Pending open/load state keyed by the ACP id returned to the client.
    pending: HashMap<String, PendingOpen>,
    /// After first bind: provisional open id → Claude native id.
    provisional_to_native: HashMap<String, String>,
    /// Process-hot duplex, if any.
    hot: Option<ClaudeDuplex>,
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

    /// ACP `initialize` result: protocol version, loadSession, curated models.
    pub(crate) fn initialize(&self) -> Value {
        json!({
            "protocolVersion": 1,
            "agentCapabilities": {
                "loadSession": true,
            },
            "_meta": {
                "modelState": {
                    "availableModels": [
                        { "modelId": "fable", "name": "Fable 5" },
                        { "modelId": "opus", "name": "Opus 4.8" },
                        { "modelId": "sonnet", "name": "Sonnet 4.6" },
                        { "modelId": "haiku", "name": "Haiku 4.5" },
                    ]
                }
            }
        })
    }

    /// Open a new ACP session without starting the official `claude` process.
    /// Returns a provisional session id; native id binds on the first prompt.
    pub(crate) async fn session_new(&mut self, params: &Value) -> Result<Value, AgentError> {
        let cwd = cwd_from_params(params);
        let model = model_from_params(params);

        // Drop any prior heat before opening a new conversation.
        if let Some(hot) = self.hot.take() {
            hot.kill().await;
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
            if hot.session_id == resolved && hot.alive() {
                self.sessions.insert(session_id);
                return Ok(json!({}));
            }
            // Wrong session or dead process — tear down; cold load records resume.
            if let Some(old) = self.hot.take() {
                old.kill().await;
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
    pub(crate) async fn run_prompt(
        &mut self,
        params: &Value,
        on_update: &mut (dyn FnMut(Value) + Send),
    ) -> Result<Value, AgentError> {
        let request_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| AgentError::InvalidParams("session/prompt missing sessionId".into()))?
            .to_string();
        let content = acp_prompt_to_claude_content(params);

        let open_id = request_id.clone();
        self.ensure_hot_and_prompt(&request_id, params, content, on_update)
            .await?;

        let live_id = self
            .hot
            .as_ref()
            .map(|h| h.session_id.clone())
            .unwrap_or_else(|| request_id.clone());
        self.sessions.insert(live_id.clone());

        let mut result = json!({ "stopReason": "end_turn" });
        if live_id != open_id {
            result["sessionId"] = json!(live_id);
        }

        Ok(result)
    }

    async fn ensure_hot_and_prompt(
        &mut self,
        request_id: &str,
        params: &Value,
        content: Vec<Value>,
        on_update: &mut (dyn FnMut(Value) + Send),
    ) -> Result<(), AgentError> {
        let resolved = self.resolve_id(request_id);

        // Reuse hot duplex when it already holds this conversation.
        if let Some(hot) = self.hot.as_mut() {
            if hot.session_id == resolved && hot.alive() {
                hot.prompt(content, on_update).await?;
                return Ok(());
            }
            if let Some(old) = self.hot.take() {
                old.kill().await;
            }
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
        let resume = pending.resume.as_deref();

        let duplex = ClaudeDuplex::open_with_first_prompt(
            &self.factory,
            &cwd,
            resume,
            model.as_deref(),
            self.bypass_permissions,
            content,
            on_update,
        )
        .await?;

        let native = duplex.session_id.clone();
        if native != request_id {
            self.provisional_to_native
                .insert(request_id.to_string(), native.clone());
        }
        self.sessions.insert(native.clone());
        self.hot = Some(duplex);
        Ok(())
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
            hot.kill().await;
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
        .unwrap_or_else(|| std::env::temp_dir())
}

fn model_from_params(params: &Value) -> Option<String> {
    // ACP clients may put model under `_meta` or top-level; accept both.
    params
        .pointer("/_meta/model")
        .or_else(|| params.get("model"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::duplex::ClaudeSpawnArgs;
    use std::process::Stdio;
    use std::sync::Arc;
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

    fn scripted_factory() -> ClaudeSpawnFactory {
        Arc::new(|args: &ClaudeSpawnArgs| {
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
        })
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
        let result = agent.run_prompt(&params, &mut sink).await?;
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
            agent.hot.as_ref().map(|h| h.session_id.as_str()),
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
            agent.hot.as_ref().map(|h| h.session_id.as_str()),
            Some(native.as_str())
        );
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1);
        agent.cancel(None).await;
    }

    /// @spec harness/claude Duplex main heat: A second main turn reuses the inner Claude process when duplex-hot
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
            agent.hot.as_ref().map(|h| h.session_id.as_str()),
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
        let err = agent
            .run_prompt(
                &json!({
                    "sessionId": "missing-session-id",
                    "prompt": [{ "type": "text", "text": "hi" }],
                }),
                &mut sink,
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

        let result = agent
            .run_prompt(
                &json!({
                    "sessionId": sid,
                    "prompt": [{ "type": "text", "text": "stream-me" }],
                }),
                &mut sink,
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
}
