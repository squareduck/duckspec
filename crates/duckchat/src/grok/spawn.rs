//! Subprocess spawning for the `grok` CLI.
//!
//! Mirrors the Claude path: every invocation goes through the user's
//! login-interactive shell so per-directory tool-manager activation (mise,
//! asdf, direnv, …) fires against the project root, rather than inheriting a
//! launchd-skeletal GUI env. The shell runs `-ilc 'exec "$@"' <argv…>` so
//! grok's args pass through positionally and `exec` hands the process image to
//! `grok` (stdin/stdout/stderr, signals, exit codes propagate directly).

use tokio::process::Command;

/// Build a `Command` that launches `grok` through the user's login shell.
/// Callers chain extra `grok` args via `.arg()` — they become positional
/// parameters to the wrapping `exec "$@"`. Set `current_dir` on the result so
/// the shell sources rcfiles with the project as its `pwd`.
pub fn grok_command() -> Command {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let mut cmd = Command::new(shell);
    cmd.arg("-ilc")
        .arg(r#"exec "$@""#)
        // $0 in the wrapper script; unused but conventional.
        .arg("duckchat-wrap")
        .arg("grok");
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_shell_around_grok() {
        let cmd = grok_command();
        let args: Vec<&str> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_str().expect("argv is ascii"))
            .collect();
        assert_eq!(args, ["-ilc", r#"exec "$@""#, "duckchat-wrap", "grok"]);
    }
}
