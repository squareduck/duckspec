//! Login-shell wrap and shared CLI flags for the official `claude` binary.
//!
//! Mirrors `duckchat::claude_code::spawn` / `run` knowledge: every production
//! invocation goes through the user's login-interactive shell so per-directory
//! tool-manager activation fires against the project root.

use std::ffi::OsString;
use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

/// Built-in CLI tools that cannot function in headless `-p` (interactive UI or
/// harness features duckboard does not provide).
///
/// `AskUserQuestion` is intentionally **not** listed: structured questions are
/// bridged to the ACP parent via the control / permission-prompt path.
pub(crate) const DISALLOWED_TOOLS: &str = "EnterPlanMode,ExitPlanMode,\
    CronCreate,CronDelete,CronList,ScheduleWakeup,RemoteTrigger,\
    PushNotification,EnterWorktree,ExitWorktree";

/// Optional direct binary override for tests (`DUCKCHAT_CLAUDE_BIN`). When set,
/// skip the login-shell wrap and spawn that path as argv[0].
pub(crate) fn claude_bin_override() -> Option<OsString> {
    std::env::var_os("DUCKCHAT_CLAUDE_BIN").filter(|v| !v.is_empty())
}

/// Argv prefix before duplex flags: either override binary alone, or
/// `SHELL -ilc 'exec "$@"' duckchat-wrap claude`.
pub(crate) fn claude_argv_prefix() -> Vec<OsString> {
    if let Some(bin) = claude_bin_override() {
        return vec![bin];
    }
    let shell = std::env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/zsh"));
    vec![
        shell,
        OsString::from("-ilc"),
        OsString::from(r#"exec "$@""#),
        OsString::from("duckchat-wrap"),
        OsString::from("claude"),
    ]
}

/// Build a `Command` for a duplex stream-json Claude session.
pub(crate) fn build_claude_command(
    cwd: &Path,
    resume: Option<&str>,
    model: Option<&str>,
    bypass_permissions: bool,
) -> Command {
    let prefix = claude_argv_prefix();
    let mut iter = prefix.into_iter();
    let program = iter.next().expect("claude argv prefix non-empty");
    let mut cmd = Command::new(program);
    for arg in iter {
        cmd.arg(arg);
    }

    cmd.arg("-p")
        .arg("--input-format")
        .arg("stream-json")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--include-partial-messages")
        .arg("--disallowedTools")
        .arg(DISALLOWED_TOOLS)
        // Route permission / canUseTool prompts through stream-json control
        // (stdio), not an interactive TTY — required for AskUserQuestion.
        .arg("--permission-prompt-tool")
        .arg("stdio")
        .arg("--settings")
        .arg(r#"{"autoMemoryEnabled":false}"#)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    if bypass_permissions {
        cmd.arg("--permission-mode").arg("bypassPermissions");
    }

    if let Some(sid) = resume {
        cmd.arg("--resume").arg(sid);
    }

    if let Some(m) = model {
        cmd.arg("--model").arg(m);
    }

    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::Mutex;

    /// Serialize env-touching tests so parallel threads do not race on
    /// `DUCKCHAT_CLAUDE_BIN`.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// @spec harness/claude Owned ACP agent over official Claude CLI: The agent uses the official claude CLI as its backend
    #[test]
    fn agent_backend_is_official_claude_cli() {
        let _guard = ENV_LOCK.lock().unwrap();
        let prev = std::env::var_os("DUCKCHAT_CLAUDE_BIN");
        unsafe { std::env::remove_var("DUCKCHAT_CLAUDE_BIN") };

        // Default production spawn: login-shell wrap of the official `claude` CLI.
        let prefix = claude_argv_prefix();
        let last = prefix.last().and_then(|s| s.to_str());
        assert_eq!(
            last,
            Some("claude"),
            "backend process must be the official claude CLI, got {prefix:?}"
        );
        assert!(
            prefix.len() >= 4,
            "expected login-shell wrap around claude: {prefix:?}"
        );

        // Full duplex command also ends with the official CLI name in argv.
        let cmd = build_claude_command(Path::new("/tmp"), None, Some("sonnet"), true);
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(
            args.iter().any(|a| a == "claude"),
            "duplex command must invoke official claude CLI: {args:?}"
        );
        assert!(
            args.iter().any(|a| a == "stream-json"),
            "agent (not host) owns the stream-json dialect: {args:?}"
        );

        if let Some(p) = prev {
            unsafe { std::env::set_var("DUCKCHAT_CLAUDE_BIN", p) };
        }
    }

    // @spec harness/claude AskUserQuestion available: AskUserQuestion is not among disallowed tools
    #[test]
    fn ask_user_question_is_not_among_disallowed_tools() {
        assert!(
            !DISALLOWED_TOOLS
                .split(',')
                .any(|t| t.trim() == "AskUserQuestion"),
            "AskUserQuestion must be allowed so Claude can issue structured questions: {DISALLOWED_TOOLS}"
        );
        // Sanity: other interactive tools stay blocked.
        assert!(DISALLOWED_TOOLS.contains("EnterPlanMode"));
    }

    #[test]
    fn spawn_enables_stdio_permission_prompt_tool() {
        let _guard = ENV_LOCK.lock().unwrap();
        let prev = std::env::var_os("DUCKCHAT_CLAUDE_BIN");
        unsafe { std::env::set_var("DUCKCHAT_CLAUDE_BIN", "/tmp/fake-claude") };
        let cmd = build_claude_command(Path::new("/tmp"), None, None, true);
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        let ppt = args.iter().position(|a| a == "--permission-prompt-tool");
        assert!(ppt.is_some(), "expected --permission-prompt-tool: {args:?}");
        assert_eq!(args[ppt.unwrap() + 1], "stdio");
        assert!(
            args.windows(2)
                .any(|w| w[0] == "--permission-mode" && w[1] == "bypassPermissions"),
            "ordinary tools stay on bypassPermissions: {args:?}"
        );
        unsafe { std::env::remove_var("DUCKCHAT_CLAUDE_BIN") };
        if let Some(p) = prev {
            unsafe { std::env::set_var("DUCKCHAT_CLAUDE_BIN", p) };
        }
    }

    #[test]
    fn override_bin_skips_shell_wrap() {
        let _guard = ENV_LOCK.lock().unwrap();
        let prev = std::env::var_os("DUCKCHAT_CLAUDE_BIN");
        unsafe { std::env::set_var("DUCKCHAT_CLAUDE_BIN", "/tmp/fake-claude") };
        let prefix = claude_argv_prefix();
        assert_eq!(prefix, vec![OsString::from("/tmp/fake-claude")]);
        unsafe { std::env::remove_var("DUCKCHAT_CLAUDE_BIN") };
        if let Some(p) = prev {
            unsafe { std::env::set_var("DUCKCHAT_CLAUDE_BIN", p) };
        }
    }
}
