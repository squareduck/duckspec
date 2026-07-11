//! Long-lived duplex `claude` child: open, prompt, cancel/kill.

use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use super::map::claude_line_to_updates;
use super::protocol::ProtocolMsg;
use super::spawn::build_claude_command;

/// Arguments used when spawning an inner Claude process.
#[derive(Debug, Clone)]
pub struct ClaudeSpawnArgs {
    pub cwd: PathBuf,
    pub resume: Option<String>,
    pub model: Option<String>,
    pub bypass_permissions: bool,
}

/// Factory for Claude child processes. Production uses the real CLI; tests
/// inject a scripted peer and can count spawns.
pub type ClaudeSpawnFactory = Arc<dyn Fn(&ClaudeSpawnArgs) -> Command + Send + Sync>;

/// Default factory: login-shell wrap of official `claude` (or `DUCKCHAT_CLAUDE_BIN`).
pub fn default_spawn_factory() -> ClaudeSpawnFactory {
    Arc::new(|args: &ClaudeSpawnArgs| {
        build_claude_command(
            &args.cwd,
            args.resume.as_deref(),
            args.model.as_deref(),
            args.bypass_permissions,
        )
    })
}

/// A counting factory wrapping an inner factory (tests).
#[cfg(test)]
pub fn counting_factory(
    inner: ClaudeSpawnFactory,
    counter: Arc<AtomicUsize>,
) -> ClaudeSpawnFactory {
    Arc::new(move |args: &ClaudeSpawnArgs| {
        counter.fetch_add(1, Ordering::SeqCst);
        inner(args)
    })
}

/// One duplex-hot Claude Code process.
pub struct ClaudeDuplex {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    /// Claude Code native session id (ACP sessionId).
    pub session_id: String,
}

#[derive(Debug)]
pub enum DuplexError {
    Spawn(String),
    Process(String),
    Protocol(String),
    SessionNotFound(String),
}

impl DuplexError {
    #[cfg(test)]
    pub fn is_session_not_found(&self) -> bool {
        matches!(self, DuplexError::SessionNotFound(_))
    }
}

impl std::fmt::Display for DuplexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DuplexError::Spawn(m)
            | DuplexError::Process(m)
            | DuplexError::Protocol(m)
            | DuplexError::SessionNotFound(m) => write!(f, "{m}"),
        }
    }
}

impl ClaudeDuplex {
    /// Spawn Claude, write the first user message, then read init + stream until
    /// `result`. Live `claude` only emits a session id after user content, so
    /// this path never waits for init before write.
    ///
    /// Profile updates are delivered via `on_update` as each Claude line is
    /// mapped (not batched until the end).
    ///
    /// `resume`: `None` for a fresh conversation; `Some(id)` for `--resume`.
    /// Missing resume sessions surface as [`DuplexError::SessionNotFound`].
    pub async fn open_with_first_prompt(
        factory: &ClaudeSpawnFactory,
        cwd: &Path,
        resume: Option<&str>,
        model: Option<&str>,
        bypass_permissions: bool,
        content: Vec<Value>,
        on_update: &mut (dyn FnMut(Value) + Send),
    ) -> Result<Self, DuplexError> {
        let args = ClaudeSpawnArgs {
            cwd: cwd.to_path_buf(),
            resume: resume.map(str::to_string),
            model: model.map(str::to_string),
            bypass_permissions,
        };
        match Self::spawn_write_and_stream(factory, args, content, on_update).await {
            Err(DuplexError::Process(m)) if resume.is_some() && looks_like_missing_session(&m) => {
                Err(DuplexError::SessionNotFound(m))
            }
            Err(DuplexError::Protocol(m)) if resume.is_some() && looks_like_missing_session(&m) => {
                Err(DuplexError::SessionNotFound(m))
            }
            other => other,
        }
    }

    /// Spawn, write user content immediately, stream updates until first `result`.
    async fn spawn_write_and_stream(
        factory: &ClaudeSpawnFactory,
        args: ClaudeSpawnArgs,
        content: Vec<Value>,
        on_update: &mut (dyn FnMut(Value) + Send),
    ) -> Result<Self, DuplexError> {
        let mut cmd = factory(&args);
        let mut child = cmd
            .spawn()
            .map_err(|e| DuplexError::Spawn(format!("failed to spawn claude: {e}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| DuplexError::Process("no stdin on claude subprocess".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| DuplexError::Process("no stdout on claude subprocess".into()))?;
        let stdout = BufReader::new(stdout);

        // Prefer the requested resume id until the stream reports a native id.
        let initial_id = args
            .resume
            .clone()
            .unwrap_or_else(|| "pending-claude".into());

        let mut duplex = Self {
            child,
            stdin,
            stdout,
            session_id: initial_id,
        };

        if let Err(e) = duplex.prompt(content, on_update).await {
            let _ = duplex.child.kill().await;
            // Resume miss often arrives as process/protocol error on first write.
            if args.resume.is_some() {
                let msg = e.to_string();
                if looks_like_missing_session(&msg) {
                    return Err(DuplexError::SessionNotFound(msg));
                }
            }
            return Err(e);
        }

        // If the stream never set a real id, keep resume id when present.
        if duplex.session_id == "pending-claude" {
            if let Some(sid) = args.resume {
                duplex.session_id = sid;
            } else {
                let _ = duplex.child.kill().await;
                return Err(DuplexError::Protocol(
                    "claude never emitted a session id during the first turn".into(),
                ));
            }
        }

        Ok(duplex)
    }

    /// Write one user message and stream mapped profile updates until `result`.
    ///
    /// Each mapped update is delivered via `on_update` as soon as its Claude
    /// line is read — before the prompt result is known.
    pub async fn prompt(
        &mut self,
        content: Vec<Value>,
        on_update: &mut (dyn FnMut(Value) + Send),
    ) -> Result<(), DuplexError> {
        let stream_msg = json!({
            "type": "user",
            "message": { "role": "user", "content": content },
        });
        let mut line = serde_json::to_string(&stream_msg)
            .map_err(|e| DuplexError::Process(format!("encode user message: {e}")))?;
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| DuplexError::Process(format!("write to claude stdin: {e}")))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| DuplexError::Process(format!("flush claude stdin: {e}")))?;

        let mut buf = String::new();
        loop {
            buf.clear();
            let n = self
                .stdout
                .read_line(&mut buf)
                .await
                .map_err(|e| DuplexError::Process(format!("read claude stdout: {e}")))?;
            if n == 0 {
                return Err(DuplexError::Process(
                    "claude closed stdout before result".into(),
                ));
            }
            let trimmed = buf.trim_end();
            if trimmed.is_empty() {
                continue;
            }
            let msg: ProtocolMsg = match serde_json::from_str(trimmed) {
                Ok(m) => m,
                Err(_) => continue,
            };

            if let Some(sid) = msg.session_id() {
                self.session_id = sid.to_string();
            }

            if msg.is_error_result() {
                let err = msg
                    .error_message()
                    .unwrap_or_else(|| "claude result error".into());
                if looks_like_missing_session(&err) {
                    return Err(DuplexError::SessionNotFound(err));
                }
                return Err(DuplexError::Process(err));
            }

            for update in claude_line_to_updates(&msg, &self.session_id) {
                on_update(update);
            }

            if msg.is_result() {
                break;
            }
        }
        Ok(())
    }

    /// Kill the inner Claude process (cancel / end heat).
    pub async fn kill(mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }

    /// True while the child has not exited.
    pub fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

fn looks_like_missing_session(msg: &str) -> bool {
    let lower = msg.to_ascii_lowercase();
    lower.contains("no such")
        || lower.contains("not found")
        || lower.contains("no conversation found")
        || lower.contains("does not exist")
        || lower.contains("unknown session")
        || lower.contains("invalid session")
        || lower.contains("session not found")
        || lower.contains("could not find")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    /// Scripted Claude peer body for `python3 -c` (stdin stays the duplex pipe).
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

    /// Factory that runs a scripted Claude peer (system init + per-user echo).
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


    #[tokio::test]
    async fn open_with_first_prompt_surfaces_native_session_id() {
        let factory = scripted_factory();
        let cwd = std::env::temp_dir();
        let mut updates = Vec::new();
        let mut sink = |u: Value| updates.push(u);
        let duplex = ClaudeDuplex::open_with_first_prompt(
            &factory,
            &cwd,
            None,
            None,
            true,
            vec![json!({"type":"text","text":"hi"})],
            &mut sink,
        )
        .await
        .expect("open with first prompt");
        assert_eq!(duplex.session_id, "claude-native-sess-1");
        assert!(
            updates
                .iter()
                .any(|u| u["update"]["sessionUpdate"] == "agent_message_chunk")
        );
        duplex.kill().await;
    }

    #[tokio::test]
    async fn second_prompt_reuses_process() {
        let counter = Arc::new(AtomicUsize::new(0));
        let factory = counting_factory(scripted_factory(), Arc::clone(&counter));
        let cwd = std::env::temp_dir();
        let mut u1 = Vec::new();
        let mut sink1 = |u: Value| u1.push(u);
        let mut duplex = ClaudeDuplex::open_with_first_prompt(
            &factory,
            &cwd,
            None,
            None,
            true,
            vec![json!({"type":"text","text":"a"})],
            &mut sink1,
        )
        .await
        .unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert!(
            u1.iter()
                .any(|u| u["update"]["content"]["text"] == "echo:a")
        );

        let mut u2 = Vec::new();
        let mut sink2 = |u: Value| u2.push(u);
        duplex
            .prompt(vec![json!({"type":"text","text":"b"})], &mut sink2)
            .await
            .unwrap();
        assert!(
            u2.iter()
                .any(|u| u["update"]["content"]["text"] == "echo:b")
        );
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "duplex-hot must not re-spawn"
        );
        duplex.kill().await;
    }

    #[tokio::test]
    async fn missing_resume_is_session_not_found() {
        let factory = scripted_factory();
        let cwd = std::env::temp_dir();
        let mut sink = |_u: Value| {};
        match ClaudeDuplex::open_with_first_prompt(
            &factory,
            &cwd,
            Some("missing-session-id"),
            None,
            true,
            vec![json!({"type":"text","text":"hi"})],
            &mut sink,
        )
        .await
        {
            Err(e) => assert!(e.is_session_not_found(), "{e}"),
            Ok(_) => panic!("expected session-not-found for missing resume id"),
        }
    }
}
