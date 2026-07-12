//! Shared ACP client can complete a turn against the owned agent binary,
//! with a scripted official-`claude` peer behind `DUCKCHAT_CLAUDE_BIN`.

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;

use duckchat::acp::{AcpTurn, AgentLaunch};
use duckchat::cancel::CancelToken;
use duckchat::event::{AgentEvent, PendingUserChoices};
use serde_json::json;
use tempfile::TempDir;
use tokio::process::Command;
use tokio::sync::mpsc;

fn agent_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_duckchat-claude-acp"))
}

/// Write a scripted Claude CLI peer that speaks stream-json duplex.
fn install_scripted_claude(dir: &TempDir) -> PathBuf {
    let path = dir.path().join("fake-claude");
    let script = r#"#!/usr/bin/env python3
import json, sys, os

# Parse --resume from argv (production spawn forwards flags after the bin).
resume = None
argv = sys.argv[1:]
if "--resume" in argv:
    i = argv.index("--resume")
    if i + 1 < len(argv):
        resume = argv[i + 1]

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
            "delta": {"type": "text_delta", "text": f"echo:{text}"},
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
    {
        let mut f = std::fs::File::create(&path).expect("create fake-claude");
        f.write_all(script.as_bytes()).unwrap();
        f.flush().unwrap();
    }
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    path
}

#[tokio::test]
async fn shared_client_completes_turn_against_agent() {
    let bin = agent_bin();
    assert!(
        bin.is_file(),
        "agent binary must exist at {}",
        bin.display()
    );

    let tmp = TempDir::new().unwrap();
    let fake_claude = install_scripted_claude(&tmp);

    let launch = AgentLaunch::new({
        let bin = bin.clone();
        let fake_claude = fake_claude.clone();
        move || {
            let mut cmd = Command::new(&bin);
            cmd.env("DUCKCHAT_CLAUDE_BIN", &fake_claude);
            cmd
        }
    });

    let cwd = std::env::temp_dir();
    let mut turn = AcpTurn::spawn_with(&launch, &cwd)
        .await
        .expect("spawn duckchat-claude-acp");

    let init = turn.initialize().await.expect("initialize");
    assert!(init.load_session);
    assert!(
        !init.models.is_empty(),
        "initialize must advertise a non-empty model catalog (live or curated fallback): {:?}",
        init.models.iter().map(|m| &m.id).collect::<Vec<_>>()
    );

    let session_id = turn.open(None, &cwd).await.expect("session/new");
    // Open returns a provisional handle; Claude is not started yet.
    assert!(
        session_id.starts_with("pending-"),
        "session/new must defer Claude spawn, got {session_id}"
    );

    // Cold load of the same handle still does not require a native id.
    let resumed = turn
        .open(Some(&session_id), &cwd)
        .await
        .expect("session/load");
    assert_eq!(resumed, session_id);

    let (tx, mut rx) = mpsc::channel::<AgentEvent>(32);
    let content = [json!({ "type": "text", "text": "ping" })];
    let pending = PendingUserChoices::shared();
    let result = turn
        .prompt_events(
            &session_id,
            &content,
            "sonnet",
            None,
            None,
            &tx,
            &CancelToken::new(),
            &pending,
        )
        .await
        .expect("session/prompt");
    assert_eq!(result.stop_reason.as_deref(), Some("end_turn"));

    drop(tx);
    let mut texts = Vec::new();
    while let Some(ev) = rx.recv().await {
        if let AgentEvent::ContentDelta { text } = ev {
            texts.push(text);
        }
    }
    assert!(
        texts.iter().any(|t| t.contains("ping")),
        "expected profile-mapped assistant text containing prompt, got {texts:?}"
    );

    turn.cancel().await;
}

#[tokio::test]
async fn missing_session_resume_fails_on_first_prompt() {
    let bin = Arc::new(agent_bin());
    let tmp = TempDir::new().unwrap();
    let fake_claude = Arc::new(install_scripted_claude(&tmp));

    let launch = AgentLaunch::new({
        let bin = Arc::clone(&bin);
        let fake_claude = Arc::clone(&fake_claude);
        move || {
            let mut cmd = Command::new(bin.as_path());
            cmd.env("DUCKCHAT_CLAUDE_BIN", fake_claude.as_path());
            cmd
        }
    });

    let cwd = std::env::temp_dir();
    let mut turn = AcpTurn::spawn_with(&launch, &cwd).await.unwrap();
    turn.initialize().await.unwrap();

    // Cold load records the id without spawning Claude.
    turn.open(Some("missing-session-id"), &cwd)
        .await
        .expect("cold session/load does not spawn");

    let (tx, _rx) = mpsc::channel::<AgentEvent>(8);
    let content = [json!({ "type": "text", "text": "hi" })];
    let pending = PendingUserChoices::shared();
    let err = turn
        .prompt_events(
            "missing-session-id",
            &content,
            "sonnet",
            None,
            None,
            &tx,
            &CancelToken::new(),
            &pending,
        )
        .await
        .expect_err("resume of missing session on first prompt");
    // Agent returns session-not-found; client may surface Protocol until step 06
    // maps prompt-time rebind errors the same way as load.
    assert!(
        err.is_session_not_found()
            || matches!(err, duckchat::Error::Protocol(ref s) if s.contains("session") || s.contains("not found") || s.contains("Path not found") || s.contains("FS_NOT_FOUND")),
        "got {err:?}"
    );
    turn.cancel().await;
}
