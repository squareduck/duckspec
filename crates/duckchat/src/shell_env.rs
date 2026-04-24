//! Background-harvested login-shell env for GUI subprocess spawns.
//!
//! When a `.app` bundle is launched from Finder / Dock, macOS launchd hands
//! us a skeletal env: minimal PATH, no `RUSTUP_TOOLCHAIN`, none of the user's
//! `.zshrc` exports. Anything that depends on a per-user tool manager (mise,
//! asdf, nix, rustup overrides, pyenv, direnv, …) then misbehaves when we
//! spawn it.
//!
//! Fix: on first init, run the user's login-interactive shell once on a
//! background thread and capture its env. Subprocess spawners overlay the
//! captured env on their `Command` via [`ShellEnvHandle::apply`], blocking
//! briefly if the harvest hasn't finished yet. Nothing touches the parent
//! process env, so this is safe under the Rust 2024 `set_var` rules without
//! main-thread ordering gymnastics.
//!
//! The approach is shell-agnostic — we don't know about mise, we just ask
//! `$SHELL` what its environment is and mirror it.
//!
//! Markers frame the `env` output so noisy `.zshrc` banners, p10k instant
//! prompts, or non-essential warnings don't corrupt the parse.

use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::{Arc, Condvar, LazyLock, Mutex};
use std::time::Duration;

/// Process-global harvester. Starts on first touch; call [`init`] from
/// `main()` to kick it off before any subprocess is spawned.
pub static SHELL_ENV: LazyLock<ShellEnvHandle> = LazyLock::new(ShellEnvHandle::spawn_harvest);

/// Force the background harvest to begin. Call from `main()` at startup so
/// the harvest runs in parallel with UI setup rather than synchronously on
/// the first subprocess spawn.
pub fn init() {
    LazyLock::force(&SHELL_ENV);
}

/// Shared handle to a (possibly in-flight) env harvest. Cheap to clone.
#[derive(Clone)]
pub struct ShellEnvHandle {
    inner: Arc<(Mutex<Option<HashMap<String, String>>>, Condvar)>,
}

impl ShellEnvHandle {
    /// Spawn a background thread that harvests the user's login-shell env
    /// and fills this handle when done. Returns immediately.
    pub fn spawn_harvest() -> Self {
        let inner = Arc::new((Mutex::new(None), Condvar::new()));
        let probe = inner.clone();
        std::thread::spawn(move || {
            let env = harvest_login_shell().unwrap_or_default();
            let (lock, cvar) = &*probe;
            let mut slot = lock.lock().expect("shell_env mutex poisoned");
            *slot = Some(env);
            cvar.notify_all();
        });
        Self { inner }
    }

    /// Construct a pre-resolved empty handle — useful for tests that want
    /// to exercise consumers without spawning a real shell.
    #[cfg(test)]
    pub fn empty() -> Self {
        let inner = Arc::new((Mutex::new(Some(HashMap::new())), Condvar::new()));
        Self { inner }
    }

    /// Overlay harvested env vars onto `cmd`. Blocks up to `timeout` for
    /// the harvest to finish; if it still isn't ready, applies nothing and
    /// the subprocess inherits whatever env the parent has (degraded but
    /// not broken).
    pub fn apply(&self, cmd: &mut Command, timeout: Duration) {
        let (lock, cvar) = &*self.inner;
        let guard = lock.lock().expect("shell_env mutex poisoned");
        let (guard, _) = cvar
            .wait_timeout_while(guard, timeout, |v| v.is_none())
            .expect("shell_env mutex poisoned during wait");
        match &*guard {
            Some(env) => {
                for (k, v) in env {
                    cmd.env(k, v);
                }
            }
            None => tracing::warn!(
                ?timeout,
                "shell env harvest not ready; subprocess inheriting parent env"
            ),
        }
    }
}

fn harvest_login_shell() -> Option<HashMap<String, String>> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    // Random-enough suffix to avoid collision with anything a user's
    // rcfile might echo.
    let marker = "__DUCKCHAT_ENV_MARKER_a21c9f__";
    let script = format!("printf '%s\\n' '{marker}'\nenv\nprintf '%s\\n' '{marker}'\n");

    let output = Command::new(&shell)
        .arg("-ilc")
        .arg(&script)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!(%shell, %e, "failed to spawn login shell for env harvest");
            return None;
        }
    };

    if !output.status.success() {
        tracing::warn!(
            %shell,
            status = %output.status,
            "login shell exited non-zero during env harvest"
        );
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = parse_marker_block(&stdout, marker);
    if parsed.is_none() {
        tracing::warn!(
            %shell,
            "login shell env harvest output missing markers; ignoring"
        );
    }
    parsed
}

fn parse_marker_block(stdout: &str, marker: &str) -> Option<HashMap<String, String>> {
    let mut lines = stdout.lines();
    lines.by_ref().find(|l| l.trim() == marker)?;

    let mut out: HashMap<String, String> = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_val = String::new();
    let mut closed = false;

    for line in lines {
        if line.trim() == marker {
            closed = true;
            break;
        }
        if let Some((k, v)) = split_env_line(line) {
            if let Some(prev) = current_key.take() {
                out.insert(prev, std::mem::take(&mut current_val));
            }
            current_key = Some(k);
            current_val = v;
        } else if current_key.is_some() {
            current_val.push('\n');
            current_val.push_str(line);
        }
    }

    if !closed {
        return None;
    }
    if let Some(prev) = current_key.take() {
        out.insert(prev, current_val);
    }
    Some(out)
}

fn split_env_line(line: &str) -> Option<(String, String)> {
    let eq = line.find('=')?;
    let (k, rest) = line.split_at(eq);
    if k.is_empty() {
        return None;
    }
    let valid = k.bytes().enumerate().all(|(i, b)| {
        if i == 0 {
            b.is_ascii_alphabetic() || b == b'_'
        } else {
            b.is_ascii_alphanumeric() || b == b'_'
        }
    });
    if !valid {
        return None;
    }
    Some((k.to_string(), rest[1..].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: &str = "__TEST_MARKER__";

    #[test]
    fn parses_simple_env_block() {
        let input = format!("{M}\nFOO=bar\nBAZ=qux\n{M}\n");
        let env = parse_marker_block(&input, M).unwrap();
        assert_eq!(env.get("FOO").map(String::as_str), Some("bar"));
        assert_eq!(env.get("BAZ").map(String::as_str), Some("qux"));
        assert_eq!(env.len(), 2);
    }

    #[test]
    fn ignores_pre_marker_noise() {
        let input = format!(
            "welcome to zsh!\nsome banner\n{M}\nFOO=bar\n{M}\n"
        );
        let env = parse_marker_block(&input, M).unwrap();
        assert_eq!(env.get("FOO").map(String::as_str), Some("bar"));
    }

    #[test]
    fn supports_multi_line_values() {
        // `env` emits newlines in values as literal newlines, with the
        // continuation line not starting with `KEY=`.
        let input = format!("{M}\nMULTI=line1\nline2\nline3\nFOO=bar\n{M}\n");
        let env = parse_marker_block(&input, M).unwrap();
        assert_eq!(
            env.get("MULTI").map(String::as_str),
            Some("line1\nline2\nline3")
        );
        assert_eq!(env.get("FOO").map(String::as_str), Some("bar"));
    }

    #[test]
    fn rejects_output_without_closing_marker() {
        let input = format!("{M}\nFOO=bar\n"); // no closing marker
        assert!(parse_marker_block(&input, M).is_none());
    }

    #[test]
    fn rejects_output_without_opening_marker() {
        let input = "FOO=bar\n".to_string();
        assert!(parse_marker_block(&input, M).is_none());
    }

    #[test]
    fn skips_lines_that_are_not_env_assignments_outside_continuations() {
        // Lines before any KEY= that aren't valid assignments and aren't
        // continuations of a previous value are just dropped.
        let input = format!("{M}\nnot a var\nstill not\nFOO=bar\n{M}\n");
        let env = parse_marker_block(&input, M).unwrap();
        assert_eq!(env.len(), 1);
        assert_eq!(env.get("FOO").map(String::as_str), Some("bar"));
    }

    #[test]
    fn handles_empty_values() {
        let input = format!("{M}\nEMPTY=\nFOO=bar\n{M}\n");
        let env = parse_marker_block(&input, M).unwrap();
        assert_eq!(env.get("EMPTY").map(String::as_str), Some(""));
        assert_eq!(env.get("FOO").map(String::as_str), Some("bar"));
    }

    #[test]
    fn split_env_line_rejects_bad_keys() {
        assert!(split_env_line("=value").is_none());
        assert!(split_env_line("1FOO=bar").is_none());
        assert!(split_env_line("FOO BAR=baz").is_none());
        assert!(split_env_line("no equals here").is_none());
        assert_eq!(
            split_env_line("FOO_BAR2=hi"),
            Some(("FOO_BAR2".into(), "hi".into()))
        );
    }

    #[test]
    fn apply_is_noop_when_harvest_times_out() {
        // Never-filled handle: apply should return quickly and not touch cmd.
        let inner = Arc::new((Mutex::new(None), Condvar::new()));
        let handle = ShellEnvHandle { inner };

        let mut cmd = Command::new("true");
        let start = std::time::Instant::now();
        handle.apply(&mut cmd, Duration::from_millis(50));
        assert!(start.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn empty_handle_applies_nothing_without_waiting() {
        let handle = ShellEnvHandle::empty();
        let mut cmd = Command::new("true");
        let start = std::time::Instant::now();
        handle.apply(&mut cmd, Duration::from_secs(10));
        assert!(start.elapsed() < Duration::from_millis(50));
    }
}
