//! Subprocess spawning for the `claude` CLI.
//!
//! Every invocation goes through the user's login-interactive shell so that
//! per-directory tool-manager activation (mise, asdf, direnv, nix-direnv,
//! pyenv, rustup overrides, …) fires against the project root. Without this,
//! `claude` inherits the host process env — which on a Finder-launched GUI is
//! launchd-skeletal — and any binary the user manages through one of those
//! tools is missing or pinned to the wrong version when claude tries to
//! spawn it.
//!
//! The shell is invoked with `-ilc 'exec "$@"' <argv…>` so claude's args
//! pass through positionally; no escaping needed. `exec` replaces the shell
//! process image with `claude`, so stdin/stdout/stderr, signals, and exit
//! codes propagate as if we spawned `claude` directly.

use std::process::Command;

/// Build a `Command` that launches `claude` through the user's login shell.
/// Callers chain extra `claude` args via `.arg()` — they become positional
/// parameters to the wrapping `exec "$@"`. Set `current_dir` on the result so
/// the shell sources rcfiles with the project as its `pwd`, which is what
/// triggers per-directory tool activation.
pub fn claude_command() -> Command {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let mut cmd = Command::new(shell);
    cmd.arg("-ilc")
        .arg(r#"exec "$@""#)
        // $0 in the wrapper script; unused but conventional.
        .arg("duckchat-wrap")
        .arg("claude");
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_shell_around_claude() {
        let cmd = claude_command();
        let args: Vec<&str> = cmd
            .get_args()
            .map(|a| a.to_str().expect("argv is ascii"))
            .collect();
        assert_eq!(args, ["-ilc", r#"exec "$@""#, "duckchat-wrap", "claude"]);
    }
}
