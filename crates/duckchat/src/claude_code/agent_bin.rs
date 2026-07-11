//! Resolve the owned `duckchat-claude-acp` agent binary for Claude turns.
//!
//! Order (first match wins):
//! 1. `DUCKCHAT_CLAUDE_ACP` environment override (selected even if missing —
//!    spawn fails later with a typed error)
//! 2. Sibling of the running executable named `duckchat-claude-acp`
//! 3. `PATH` lookup for `duckchat-claude-acp`

use std::env;
use std::path::{Path, PathBuf};

use crate::acp::AgentLaunch;
use crate::error::Error;

/// Environment variable that forces a specific Claude ACP agent binary path.
pub const CLAUDE_ACP_ENV: &str = "DUCKCHAT_CLAUDE_ACP";

/// Binary name of the owned Claude ACP agent.
pub const CLAUDE_ACP_BIN: &str = "duckchat-claude-acp";

/// Resolve the Claude ACP agent binary path.
///
/// Returns [`Error::Spawn`] when no override is set and neither a sibling nor
/// a `PATH` entry is available.
pub fn resolve_claude_acp_binary() -> Result<PathBuf, Error> {
    let env_override = env::var_os(CLAUDE_ACP_ENV).filter(|v| !v.is_empty());
    let current_exe = env::current_exe().ok();
    resolve_with(
        env_override.as_ref().map(PathBuf::from),
        current_exe.as_deref(),
        path_lookup,
    )
}

/// Build an [`AgentLaunch`] that spawns the resolved Claude ACP agent.
///
/// Resolution runs at spawn time so env/sibling/`PATH` stay live. When no
/// binary can be resolved, the launch still produces a command that fails
/// spawn with a typed error (same class as a missing `grok`).
pub fn claude_acp_launch() -> AgentLaunch {
    AgentLaunch::new(|| {
        let path = resolve_claude_acp_binary()
            .unwrap_or_else(|_| PathBuf::from(CLAUDE_ACP_BIN));
        tokio::process::Command::new(path)
    })
}

/// Testable resolution core.
///
/// - `env_override`: value of `DUCKCHAT_CLAUDE_ACP` when set
/// - `current_exe`: path of the running executable (for sibling search)
/// - `path_lookup`: `PATH` search for [`CLAUDE_ACP_BIN`]
pub(crate) fn resolve_with(
    env_override: Option<PathBuf>,
    current_exe: Option<&Path>,
    path_lookup: impl FnOnce(&str) -> Option<PathBuf>,
) -> Result<PathBuf, Error> {
    if let Some(path) = env_override {
        return Ok(path);
    }

    if let Some(exe) = current_exe
        && let Some(dir) = exe.parent()
    {
        let sibling = dir.join(CLAUDE_ACP_BIN);
        if sibling.is_file() {
            return Ok(sibling);
        }
    }

    if let Some(path) = path_lookup(CLAUDE_ACP_BIN) {
        return Ok(path);
    }

    Err(Error::Spawn(format!(
        "{CLAUDE_ACP_BIN} not found: set {CLAUDE_ACP_ENV}, place the binary next to the running executable, or install it on PATH"
    )))
}

/// Search `PATH` for an executable named `name`.
fn path_lookup(name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::AcpTurn;
    use crate::cancel::CancelToken;
    use crate::request::TurnRequest;
    use crate::runtime::MainRuntime;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    /// Serialize env-touching tests so parallel test threads do not race on
    /// `DUCKCHAT_CLAUDE_ACP` / synthetic exe layouts.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn touch_executable(path: &Path) {
        fs::write(path, b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms).unwrap();
        }
    }

    /// @spec harness/claude Agent binary discovery: An explicit env override selects the agent binary
    #[test]
    fn env_override_selects_agent_binary() {
        let _guard = ENV_LOCK.lock().unwrap();
        let override_path = PathBuf::from("/explicit/path/to/duckchat-claude-acp");
        let resolved = resolve_with(
            Some(override_path.clone()),
            Some(Path::new("/app/duckboard")),
            |_| Some(PathBuf::from("/usr/bin/duckchat-claude-acp")),
        )
        .unwrap();
        assert_eq!(resolved, override_path);
    }

    /// @spec harness/claude Agent binary discovery: When env is unset, a sibling of the running executable is used if present
    #[test]
    fn sibling_of_executable_is_used_when_present() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let exe = dir.path().join("duckboard");
        let sibling = dir.path().join(CLAUDE_ACP_BIN);
        touch_executable(&exe);
        touch_executable(&sibling);

        let resolved = resolve_with(None, Some(&exe), |_| {
            panic!("PATH must not be consulted when a sibling exists")
        })
        .unwrap();
        assert_eq!(resolved, sibling);
    }

    /// @spec harness/claude Agent binary discovery: A missing agent binary fails the turn with a typed error
    #[tokio::test]
    async fn missing_agent_binary_fails_turn_with_typed_error() {
        {
            let _guard = ENV_LOCK.lock().unwrap();
            // No env override, no sibling, no PATH hit → resolve fails with Spawn.
            let err = resolve_with(None, Some(Path::new("/no/such/duckboard")), |_| None)
                .unwrap_err();
            assert!(matches!(err, Error::Spawn(_)), "got {err:?}");
        }

        // A turn that tries to spawn a non-existent agent fails the same way
        // (typed error, no panic) — same operator class as a missing grok.
        let launch = AgentLaunch::new(|| {
            tokio::process::Command::new("/nonexistent/duckchat-claude-acp-missing")
        });
        let mut runtime = crate::acp::AcpMainRuntime::new(launch, &std::env::temp_dir());
        let (tx, _rx) = mpsc::channel(8);
        let req = TurnRequest::new("hello", std::env::temp_dir());
        let outcome = runtime
            .run_turn(
                req,
                tx,
                CancelToken::new(),
                crate::event::PendingUserChoices::shared(),
            )
            .await;
        assert!(
            matches!(outcome, Err(Error::Spawn(_))),
            "expected Spawn error, got {outcome:?}"
        );

        // Direct spawn path also types the failure.
        let launch = AgentLaunch::new(|| {
            tokio::process::Command::new("/nonexistent/duckchat-claude-acp-missing")
        });
        let spawn = AcpTurn::spawn_with(&launch, &std::env::temp_dir()).await;
        assert!(matches!(spawn, Err(Error::Spawn(_))));
    }

    #[test]
    fn path_lookup_used_when_no_env_or_sibling() {
        let _guard = ENV_LOCK.lock().unwrap();
        let path_hit = PathBuf::from("/opt/bin/duckchat-claude-acp");
        let resolved = resolve_with(None, Some(Path::new("/app/duckboard")), |_| {
            Some(path_hit.clone())
        })
        .unwrap();
        assert_eq!(resolved, path_hit);
    }
}
