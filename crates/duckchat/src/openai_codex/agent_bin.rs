//! Resolve the owned `duckchat-codex-acp` agent binary for Codex turns.
//!
//! Order (first match wins):
//! 1. `DUCKCHAT_CODEX_ACP` environment override (selected even if missing —
//!    spawn fails later with a typed error)
//! 2. Sibling of the running executable named `duckchat-codex-acp`
//! 3. `PATH` lookup for `duckchat-codex-acp`

use std::env;
use std::path::{Path, PathBuf};

use crate::acp::AgentLaunch;
use crate::error::Error;

/// Environment variable that forces a specific Codex ACP agent binary path.
pub const CODEX_ACP_ENV: &str = "DUCKCHAT_CODEX_ACP";

/// Binary name of the owned Codex ACP agent.
pub const CODEX_ACP_BIN: &str = "duckchat-codex-acp";

/// Resolve the Codex ACP agent binary path.
///
/// Returns [`Error::Spawn`] when no override is set and neither a sibling nor
/// a `PATH` entry is available.
pub fn resolve_codex_acp_binary() -> Result<PathBuf, Error> {
    let env_override = env::var_os(CODEX_ACP_ENV).filter(|v| !v.is_empty());
    let current_exe = env::current_exe().ok();
    resolve_with(
        env_override.as_ref().map(PathBuf::from),
        current_exe.as_deref(),
        path_lookup,
    )
}

/// Build an [`AgentLaunch`] that spawns the resolved Codex ACP agent.
///
/// Resolution runs at spawn time so env/sibling/`PATH` stay live. When no
/// binary can be resolved, the launch still produces a command that fails
/// spawn with a typed error (same class as a missing Claude agent).
pub fn codex_acp_launch() -> AgentLaunch {
    AgentLaunch::new(|| {
        let path = resolve_codex_acp_binary().unwrap_or_else(|_| PathBuf::from(CODEX_ACP_BIN));
        tokio::process::Command::new(path)
    })
}

/// Testable resolution core.
///
/// - `env_override`: value of `DUCKCHAT_CODEX_ACP` when set
/// - `current_exe`: path of the running executable (for sibling search)
/// - `path_lookup`: `PATH` search for [`CODEX_ACP_BIN`]
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
        let sibling = dir.join(CODEX_ACP_BIN);
        if sibling.is_file() {
            return Ok(sibling);
        }
    }

    if let Some(path) = path_lookup(CODEX_ACP_BIN) {
        return Ok(path);
    }

    Err(Error::Spawn(format!(
        "{CODEX_ACP_BIN} not found: set {CODEX_ACP_ENV}, place the binary next to the running executable, or install it on PATH"
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
    /// `DUCKCHAT_CODEX_ACP` / synthetic exe layouts.
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

    /// @spec harness/openai-codex Agent binary discovery: An explicit env override selects the agent binary
    #[test]
    fn env_override_selects_agent_binary() {
        let _guard = ENV_LOCK.lock().unwrap();
        let override_path = PathBuf::from("/explicit/path/to/duckchat-codex-acp");
        let resolved = resolve_with(
            Some(override_path.clone()),
            Some(Path::new("/app/duckboard")),
            |_| Some(PathBuf::from("/usr/bin/duckchat-codex-acp")),
        )
        .unwrap();
        assert_eq!(resolved, override_path);
    }

    /// @spec harness/openai-codex Agent binary discovery: When env is unset, a sibling of the running executable is used if present
    #[test]
    fn sibling_of_executable_is_used_when_present() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let exe = dir.path().join("duckboard");
        let sibling = dir.path().join(CODEX_ACP_BIN);
        touch_executable(&exe);
        touch_executable(&sibling);

        let resolved = resolve_with(None, Some(&exe), |_| {
            panic!("PATH must not be consulted when a sibling exists")
        })
        .unwrap();
        assert_eq!(resolved, sibling);
    }

    /// @spec harness/openai-codex Agent binary discovery: A missing agent binary fails the turn with a typed error
    #[tokio::test]
    async fn missing_agent_binary_fails_turn_with_typed_error() {
        {
            let _guard = ENV_LOCK.lock().unwrap();
            // No env override, no sibling, no PATH hit → resolve fails with Spawn.
            let err =
                resolve_with(None, Some(Path::new("/no/such/duckboard")), |_| None).unwrap_err();
            assert!(matches!(err, Error::Spawn(_)), "got {err:?}");
        }

        // A turn that tries to spawn a non-existent agent fails the same way
        // (typed error, no panic) — same operator class as a missing Claude agent.
        let launch = AgentLaunch::new(|| {
            tokio::process::Command::new("/nonexistent/duckchat-codex-acp-missing")
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
            tokio::process::Command::new("/nonexistent/duckchat-codex-acp-missing")
        });
        let spawn = AcpTurn::spawn_with(&launch, &std::env::temp_dir()).await;
        assert!(matches!(spawn, Err(Error::Spawn(_))));
    }

    /// @spec harness/openai-codex Owned ACP agent over official Codex: The harness does not require a Node or npm runtime
    #[test]
    fn harness_does_not_require_node_or_npm_runtime() {
        let _guard = ENV_LOCK.lock().unwrap();
        // Resolution always yields the owned native agent binary name/path —
        // never node, npm, or npx as the program to spawn for a Codex turn.
        let override_path = PathBuf::from("/opt/bin/duckchat-codex-acp");
        let resolved = resolve_with(Some(override_path.clone()), None, |_| {
            panic!("PATH must not be consulted when env override is set")
        })
        .unwrap();
        assert_eq!(resolved, override_path);
        let program = resolved
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        assert_eq!(program, CODEX_ACP_BIN);
        assert_ne!(program, "node");
        assert_ne!(program, "npm");
        assert_ne!(program, "npx");

        // PATH fallback is the same native binary name.
        let path_hit = PathBuf::from("/usr/local/bin/duckchat-codex-acp");
        let from_path = resolve_with(None, Some(Path::new("/app/duckboard")), |_| {
            Some(path_hit.clone())
        })
        .unwrap();
        assert_eq!(from_path, path_hit);
        assert!(
            from_path.file_name().is_some_and(|n| n == CODEX_ACP_BIN),
            "PATH resolution must target the native agent, not a JS runtime"
        );

        // Launch builds a command whose program is the agent binary (not npx).
        let launch = codex_acp_launch();
        let cmd = launch.command();
        let program = cmd.as_std().get_program().to_string_lossy();
        assert!(
            !program.contains("node") && !program.contains("npm") && !program.contains("npx"),
            "launch program must not be a Node/npm runtime: {program}"
        );
        assert!(
            program.contains(CODEX_ACP_BIN) || program.ends_with(CODEX_ACP_BIN),
            "launch program should be the owned agent binary: {program}"
        );
    }

    #[test]
    fn path_lookup_used_when_no_env_or_sibling() {
        let _guard = ENV_LOCK.lock().unwrap();
        let path_hit = PathBuf::from("/opt/bin/duckchat-codex-acp");
        let resolved = resolve_with(None, Some(Path::new("/app/duckboard")), |_| {
            Some(path_hit.clone())
        })
        .unwrap();
        assert_eq!(resolved, path_hit);
    }
}
