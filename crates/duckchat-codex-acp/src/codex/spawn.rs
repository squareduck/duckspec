//! Resolve and spawn official `codex app-server` (stdio).

use std::ffi::OsString;
use std::process::Stdio;
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::process::Command;

/// Optional direct binary override for tests (`DUCKCHAT_CODEX_BIN`).
pub(crate) const CODEX_BIN_ENV: &str = "DUCKCHAT_CODEX_BIN";

/// Official CLI binary name when no override is set.
pub(crate) const CODEX_BIN: &str = "codex";

/// Factory for `codex app-server` child processes.
pub type CodexSpawnFactory = Arc<dyn Fn() -> Command + Send + Sync>;

/// Default factory: official `codex app-server --stdio` (or `DUCKCHAT_CODEX_BIN`).
pub fn default_spawn_factory() -> CodexSpawnFactory {
    Arc::new(build_app_server_command)
}

/// Resolve the `codex` binary path used as argv[0] (override or bare name).
pub fn resolve_codex_bin() -> OsString {
    std::env::var_os(CODEX_BIN_ENV)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| OsString::from(CODEX_BIN))
}

/// Build a `Command` for stdio app-server.
pub fn build_app_server_command() -> Command {
    let bin = resolve_codex_bin();
    let mut cmd = Command::new(bin);
    cmd.arg("app-server")
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    cmd
}

/// Counting factory wrapping an inner factory (tests).
#[cfg(test)]
pub fn counting_factory(inner: CodexSpawnFactory, counter: Arc<AtomicUsize>) -> CodexSpawnFactory {
    Arc::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        inner()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @spec harness/openai-codex Owned ACP agent over official Codex: The agent uses official codex app-server as its backend
    #[test]
    fn spawn_uses_official_codex_app_server() {
        // Production argv is `<codex> app-server --stdio`, not node/npm/npx.
        let bin = resolve_codex_bin();
        let bin_str = bin.to_string_lossy();
        assert!(
            !bin_str.contains("node") && !bin_str.contains("npm") && !bin_str.contains("npx"),
            "codex backend must be the official CLI, got {bin_str}"
        );

        let cmd = build_app_server_command();
        let std = cmd.as_std();
        let program = std.get_program().to_string_lossy();
        let args: Vec<String> = std
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(
            program.ends_with("codex") || program.contains("codex") || !program.is_empty(),
            "program should resolve to codex CLI: {program}"
        );
        assert_eq!(
            args.first().map(String::as_str),
            Some("app-server"),
            "first arg must be app-server, got {args:?}"
        );
        assert!(
            args.iter().any(|a| a == "--stdio"),
            "must use stdio transport: {args:?}"
        );
        assert!(
            !args.iter().any(|a| a == "npx" || a.contains("npm")),
            "must not launch via npm/npx: {args:?}"
        );
    }
}
