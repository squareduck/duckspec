//! Per-scope chat session model and persistence.
//!
//! A "scope" is a change name, "caps", or "codex". Each scope can have multiple
//! chat sessions, stored under `<data>/chats/<scope>/<session_id>.json`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

// ── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: Vec<ContentBlock>,
    pub timestamp: String,
    /// Synthetic priming turn injected by the harness — currently the
    /// first-turn AGENTS.md inject. Excluded from `title_summarization_target`
    /// so the title summariser keys off the user's actual intent, not
    /// boilerplate project conventions. Defaults to false on load for
    /// backwards compatibility with sessions persisted before this field
    /// existed.
    #[serde(default)]
    pub is_priming: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentBlock {
    Text(String),
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    ToolResult {
        id: String,
        name: String,
        output: String,
    },
}

/// In-memory chat session.
#[derive(Debug, Clone)]
pub struct ChatSession {
    pub id: String,
    pub scope: String,
    pub created_at_nanos: i128,
    pub display_name: String,
    pub messages: Vec<ChatMessage>,
    pub is_streaming: bool,
    pub pending_text: String,
    /// Claude Code CLI session id, used with `--resume` for multi-turn continuity.
    /// Set after the first successful turn; persisted so conversations can be
    /// resumed across app restarts.
    pub claude_session_id: Option<String>,
    /// Short summary produced by the title hook after the first
    /// user/assistant exchange. `Some` for change sessions that have been
    /// summarised; `None` otherwise (including all exploration/caps/codex
    /// sessions — explorations store their title on the Exploration itself,
    /// caps/codex don't summarise). Also used as a "don't resummarise" flag.
    pub title: Option<String>,
    /// The idea body most recently injected into this chat's system context.
    /// Persisted so we only re-inject when the idea body actually changes
    /// between turns (first turn always injects when non-empty).
    pub last_seeded_description: Option<String>,
}

impl ChatSession {
    /// Create a brand-new session scoped to `scope` (change name / "caps" / "codex").
    /// The `display_name` is a base "YYYY-MM-DD HH:MM <scope>" without any
    /// collision suffix; call `reconcile_display_names` after the session is
    /// inserted into its sibling list to apply `#N` suffixes as needed.
    pub fn new(scope: String) -> Self {
        let now = current_local_datetime();
        let created_at_nanos = now.unix_timestamp_nanos();
        let id = created_at_nanos.to_string();
        let display_name = base_display_name(now, &scope);
        Self {
            id,
            scope,
            created_at_nanos,
            display_name,
            messages: Vec::new(),
            is_streaming: false,
            pending_text: String::new(),
            claude_session_id: None,
            title: None,
            last_seeded_description: None,
        }
    }
}

/// On-disk format. `display_name` is recomputed on load from the timestamp
/// plus the scope, so it doesn't need to be persisted.
#[derive(Serialize, Deserialize)]
struct PersistedSession {
    id: String,
    created_at_nanos: i128,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    claude_session_id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    last_seeded_description: Option<String>,
}

/// What the title summariser should summarise. A "bare slash command" turn
/// (e.g. just `/ds-apply` with no trailing content) is skipped as the
/// summarisation target because the message itself carries no intent —
/// instead we wait for a turn with actual content. The most recent slash
/// command seen up to that point is carried in `command_hint_source` so
/// callers can still attach the matching `title_hints` hint.
pub struct TitleTarget {
    /// The user message whose text will be summarised. Always non-empty and
    /// never a bare slash command.
    pub message: String,
    /// The user-message text (bare command or otherwise) that contributed
    /// the most recent `/<cmd>` seen up to and including `message`. `None`
    /// when no slash command has been sent yet.
    pub command_hint_source: Option<String>,
}

/// Walk the session's user turns (skipping priming) and decide whether the
/// title summariser should fire. Returns `None` when every user turn so far
/// is a bare slash command — the caller defers summarisation until a real
/// message arrives. Otherwise returns the first non-bare-command message
/// together with the most recent slash-command context.
pub fn title_summarization_target(session: &ChatSession) -> Option<TitleTarget> {
    let mut last_command_source: Option<String> = None;
    for msg in &session.messages {
        if !matches!(msg.role, Role::User) || msg.is_priming {
            continue;
        }
        let Some(text) = msg.content.iter().find_map(|b| match b {
            ContentBlock::Text(t) => Some(t.clone()),
            _ => None,
        }) else {
            continue;
        };
        if starts_with_slash_command(&text) {
            last_command_source = Some(text.clone());
        }
        if is_bare_slash_command(&text) {
            continue;
        }
        return Some(TitleTarget {
            message: text,
            command_hint_source: last_command_source,
        });
    }
    None
}

/// True when `text` starts with a `/` followed by a non-empty token. Used
/// to detect command-bearing turns regardless of whether they have content
/// after the command.
fn starts_with_slash_command(text: &str) -> bool {
    let trimmed = text.trim_start();
    let Some(rest) = trimmed.strip_prefix('/') else {
        return false;
    };
    rest.chars()
        .next()
        .is_some_and(|c| !c.is_whitespace())
}

/// True when `text` is exactly a slash command with no trailing content
/// (e.g. `/ds-apply`, `   /ds-apply  `). Leading/trailing whitespace is
/// ignored; anything else after the command token makes it non-bare.
pub fn is_bare_slash_command(text: &str) -> bool {
    let trimmed = text.trim();
    let Some(rest) = trimmed.strip_prefix('/') else {
        return false;
    };
    !rest.is_empty() && !rest.chars().any(char::is_whitespace)
}

/// Recompute `display_name` on every session so that sessions sharing the
/// same minute-prefix get `#1`, `#2`, ... suffixes in chronological order,
/// and singletons have no suffix.
///
/// `scope_label` is the human-readable label for this scope (change name,
/// exploration display_name, or "caps"/"codex") — used when the session
/// hasn't been summarised yet. Sessions with `title` set use that instead.
pub fn reconcile_display_names(sessions: &mut [ChatSession], scope_label: &str) {
    use std::collections::HashMap;
    let label_for = |s: &ChatSession| -> String {
        s.title.clone().unwrap_or_else(|| scope_label.to_string())
    };
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, s) in sessions.iter().enumerate() {
        let prefix = minute_prefix_from_nanos(s.created_at_nanos);
        groups.entry(prefix).or_default().push(i);
    }
    for (_prefix, mut indices) in groups {
        indices.sort_by_key(|&i| sessions[i].created_at_nanos);
        if indices.len() == 1 {
            let i = indices[0];
            let minute = minute_prefix_from_nanos(sessions[i].created_at_nanos);
            let label = label_for(&sessions[i]);
            sessions[i].display_name = format!("{minute} {label}");
        } else {
            for (n, i) in indices.iter().enumerate() {
                let minute = minute_prefix_from_nanos(sessions[*i].created_at_nanos);
                let label = label_for(&sessions[*i]);
                sessions[*i].display_name = format!("{minute} #{} {label}", n + 1);
            }
        }
    }
}

// ── Time helpers ────────────────────────────────────────────────────────────

fn current_local_datetime() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

fn base_display_name(dt: OffsetDateTime, scope: &str) -> String {
    format!("{} {}", minute_prefix(dt), scope)
}

fn minute_prefix(dt: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        dt.year(),
        dt.month() as u8,
        dt.day(),
        dt.hour(),
        dt.minute(),
    )
}

pub fn minute_prefix_public(nanos: i128) -> String {
    minute_prefix_from_nanos(nanos)
}

fn minute_prefix_from_nanos(nanos: i128) -> String {
    let dt = OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .ok()
        .and_then(|utc| {
            time::UtcOffset::current_local_offset()
                .ok()
                .map(|off| utc.to_offset(off))
        })
        .unwrap_or_else(|| {
            OffsetDateTime::from_unix_timestamp_nanos(nanos).unwrap_or(OffsetDateTime::UNIX_EPOCH)
        });
    minute_prefix(dt)
}

// ── Paths ───────────────────────────────────────────────────────────────────

fn chats_root(project_root: Option<&Path>) -> PathBuf {
    crate::config::data_dir(project_root).join("chats")
}

fn scope_dir(scope: &str, project_root: Option<&Path>) -> PathBuf {
    chats_root(project_root).join(scope)
}

fn session_path(scope: &str, session_id: &str, project_root: Option<&Path>) -> PathBuf {
    scope_dir(scope, project_root).join(format!("{session_id}.json"))
}

// ── Load / save ─────────────────────────────────────────────────────────────

/// Load all sessions for a scope, sorted newest-first.
pub fn load_sessions_for(scope: &str, project_root: Option<&Path>) -> Vec<ChatSession> {
    let dir = scope_dir(scope, project_root);
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut sessions = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Ok(data) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(persisted) = serde_json::from_str::<PersistedSession>(&data) else {
            continue;
        };
        sessions.push(ChatSession {
            id: persisted.id,
            scope: scope.to_string(),
            created_at_nanos: persisted.created_at_nanos,
            display_name: String::new(),
            messages: persisted.messages,
            is_streaming: false,
            pending_text: String::new(),
            claude_session_id: persisted.claude_session_id,
            title: persisted.title,
            last_seeded_description: persisted.last_seeded_description,
        });
    }
    sessions.sort_by(|a, b| b.created_at_nanos.cmp(&a.created_at_nanos));
    // At load time we don't yet have the caller's preferred label (exploration
    // display_name may differ from scope key). Use the scope key as a
    // placeholder; callers re-reconcile with the right label afterwards.
    reconcile_display_names(&mut sessions, scope);
    sessions
}

/// Save a session to disk under `chats/<scope>/<id>.json`.
pub fn save_session(session: &ChatSession, project_root: Option<&Path>) -> anyhow::Result<()> {
    let dir = scope_dir(&session.scope, project_root);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", session.id));
    let persisted = PersistedSession {
        id: session.id.clone(),
        created_at_nanos: session.created_at_nanos,
        messages: session.messages.clone(),
        claude_session_id: session.claude_session_id.clone(),
        title: session.title.clone(),
        last_seeded_description: session.last_seeded_description.clone(),
    };
    let data = serde_json::to_string_pretty(&persisted)?;
    std::fs::write(path, data)?;
    Ok(())
}

/// Delete a single session file.
pub fn delete_session(scope: &str, session_id: &str, project_root: Option<&Path>) {
    if let Err(e) = std::fs::remove_file(session_path(scope, session_id, project_root))
        && e.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(scope, session_id, "failed to delete session file: {e}");
    }
}

/// Delete all sessions for a scope (directory removal).
pub fn delete_scope(scope: &str, project_root: Option<&Path>) {
    if let Err(e) = std::fs::remove_dir_all(scope_dir(scope, project_root))
        && e.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(scope, "failed to delete scope directory: {e}");
    }
}

/// Count `.json` session files for a scope. Returns 0 when the scope dir
/// is missing or unreadable.
pub fn count_sessions(scope: &str, project_root: Option<&Path>) -> usize {
    let dir = scope_dir(scope, project_root);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return 0;
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .count()
}

/// Wipe the entire per-project data directory (chats, ideas, explorations.json,
/// anything future). Idempotent — missing dirs are not an error.
pub fn delete_project_data(project_root: &Path) {
    let dir = crate::config::data_dir(Some(project_root));
    if let Err(e) = std::fs::remove_dir_all(&dir)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(path = %dir.display(), "failed to delete project data: {e}");
    }
}

/// Rename a scope directory: `chats/<old>` → `chats/<new>`.
pub fn rename_scope(old: &str, new: &str, project_root: Option<&Path>) {
    let old_dir = scope_dir(old, project_root);
    let new_dir = scope_dir(new, project_root);
    if old_dir.exists()
        && let Err(e) = std::fs::rename(&old_dir, &new_dir)
    {
        tracing::warn!(
            from = old,
            to = new,
            "failed to rename scope directory: {e}"
        );
    }
}

// ── Exploration persistence ─────────────────────────────────────────────────

/// An exploration tracks a free-form chat scope that may eventually be promoted
/// to a real change. `id` is the stable directory key for `chats/<id>/`;
/// `display_name` is what the UI shows and can be updated by the title
/// summariser without moving the chat directory. `idea_path` backlinks to
/// the idea file when the exploration was started from one — idea-owned
/// explorations are hidden from the Changes area list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exploration {
    pub id: String,
    pub display_name: String,
    #[serde(default, alias = "card_id")]
    pub idea_path: Option<String>,
    /// Transient cache of `count_sessions(id, ...)` so the UI can decide
    /// whether to arm the destructive close button without `read_dir`-ing
    /// on every redraw. Repopulated by `load_explorations` and
    /// `recount_explorations`; never serialised.
    #[serde(skip)]
    pub session_count: usize,
}

impl Exploration {
    /// Mint a new exploration with a stable, timestamp-based id. `counter` is
    /// only used to seed the default display_name — the id is derived from
    /// the wall clock so two quick-fire creates don't collide.
    pub fn new(counter: usize) -> Self {
        let nanos = current_local_datetime().unix_timestamp_nanos();
        Self {
            id: format!("exploration-{nanos}"),
            display_name: format!("Exploration {counter}"),
            idea_path: None,
            session_count: 0,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ExplorationData {
    explorations: Vec<Exploration>,
    counter: usize,
}

pub fn load_explorations(project_root: Option<&Path>) -> (Vec<Exploration>, usize) {
    let path = crate::config::data_dir(project_root).join("explorations.json");
    let Ok(data) = std::fs::read_to_string(&path) else {
        return (vec![], 0);
    };
    let Ok(state) = serde_json::from_str::<ExplorationData>(&data) else {
        return (vec![], 0);
    };
    let mut explorations = state.explorations;
    recount_explorations(&mut explorations, project_root);
    (explorations, state.counter)
}

/// Refresh `session_count` on every exploration via `count_sessions`. Cheap
/// (one `read_dir` per exploration); call after any message that may have
/// added or removed a session, so the UI's arm-or-skip decision stays
/// accurate without per-frame I/O.
pub fn recount_explorations(explorations: &mut [Exploration], project_root: Option<&Path>) {
    for exp in explorations.iter_mut() {
        exp.session_count = count_sessions(&exp.id, project_root);
    }
}

pub fn save_explorations(
    explorations: &[Exploration],
    counter: usize,
    project_root: Option<&Path>,
) {
    let dir = crate::config::data_dir(project_root);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::warn!("failed to create explorations directory: {e}");
        return;
    }
    match serde_json::to_string_pretty(&ExplorationData {
        explorations: explorations.to_vec(),
        counter,
    }) {
        Ok(data) => {
            if let Err(e) = std::fs::write(dir.join("explorations.json"), data) {
                tracing::warn!("failed to write explorations.json: {e}");
            }
        }
        Err(e) => tracing::warn!("failed to serialize explorations: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_msg(text: &str) -> ChatMessage {
        ChatMessage {
            role: Role::User,
            content: vec![ContentBlock::Text(text.into())],
            timestamp: String::new(),
            is_priming: false,
        }
    }

    fn priming_msg(text: &str) -> ChatMessage {
        ChatMessage {
            role: Role::User,
            content: vec![ContentBlock::Text(text.into())],
            timestamp: String::new(),
            is_priming: true,
        }
    }

    fn assistant_msg(text: &str) -> ChatMessage {
        ChatMessage {
            role: Role::Assistant,
            content: vec![ContentBlock::Text(text.into())],
            timestamp: String::new(),
            is_priming: false,
        }
    }

    fn session_with(messages: Vec<ChatMessage>) -> ChatSession {
        let mut s = ChatSession::new("test".into());
        s.messages = messages;
        s
    }

    #[test]
    fn is_bare_slash_command_basics() {
        assert!(is_bare_slash_command("/ds-apply"));
        assert!(is_bare_slash_command("  /ds-apply  "));
        assert!(!is_bare_slash_command("/ds-apply now"));
        assert!(!is_bare_slash_command("hello"));
        assert!(!is_bare_slash_command(""));
        assert!(!is_bare_slash_command("/"));
        assert!(!is_bare_slash_command("/ next"));
    }

    #[test]
    fn target_none_when_empty_or_only_priming() {
        assert!(title_summarization_target(&session_with(vec![])).is_none());
        assert!(
            title_summarization_target(&session_with(vec![priming_msg("AGENTS.md ...")])).is_none()
        );
    }

    #[test]
    fn target_none_when_only_bare_commands() {
        let s = session_with(vec![user_msg("/ds-apply"), assistant_msg("...")]);
        assert!(title_summarization_target(&s).is_none());
    }

    #[test]
    fn target_returns_first_real_message_no_command_history() {
        let s = session_with(vec![user_msg("wire up the login form")]);
        let t = title_summarization_target(&s).unwrap();
        assert_eq!(t.message, "wire up the login form");
        assert!(t.command_hint_source.is_none());
    }

    #[test]
    fn target_carries_prior_bare_command_as_hint_source() {
        let s = session_with(vec![
            user_msg("/ds-apply"),
            assistant_msg("ok"),
            user_msg("now wire up the form"),
        ]);
        let t = title_summarization_target(&s).unwrap();
        assert_eq!(t.message, "now wire up the form");
        assert_eq!(t.command_hint_source.as_deref(), Some("/ds-apply"));
    }

    #[test]
    fn target_uses_most_recent_bare_command() {
        let s = session_with(vec![
            user_msg("/ds-apply"),
            user_msg("/ds-verify"),
            user_msg("ok continue"),
        ]);
        let t = title_summarization_target(&s).unwrap();
        assert_eq!(t.message, "ok continue");
        assert_eq!(t.command_hint_source.as_deref(), Some("/ds-verify"));
    }

    #[test]
    fn target_self_references_when_message_leads_with_command() {
        let s = session_with(vec![user_msg("/ds-apply look at the tests too")]);
        let t = title_summarization_target(&s).unwrap();
        assert_eq!(t.message, "/ds-apply look at the tests too");
        assert_eq!(
            t.command_hint_source.as_deref(),
            Some("/ds-apply look at the tests too")
        );
    }

    #[test]
    fn target_skips_priming_messages() {
        let s = session_with(vec![
            priming_msg("AGENTS.md ..."),
            user_msg("/ds-apply"),
            user_msg("real text"),
        ]);
        let t = title_summarization_target(&s).unwrap();
        assert_eq!(t.message, "real text");
        assert_eq!(t.command_hint_source.as_deref(), Some("/ds-apply"));
    }

    // ── session count / delete project data ─────────────────────────────

    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static FS_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct FsTmp(std::path::PathBuf);

    impl FsTmp {
        fn new() -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let counter = FS_COUNTER.fetch_add(1, Ordering::SeqCst);
            let mut p = std::env::temp_dir();
            p.push(format!("duckboard-chat-store-test-{nanos}-{counter}"));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for FsTmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Set XDG-equivalent `HOME` so `config::data_dir` resolves under our
    /// temp dir. Two tests run in parallel can collide if they share `HOME`;
    /// each test serialises through the `HOME_LOCK` mutex.
    static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_home<R>(home: &std::path::Path, f: impl FnOnce() -> R) -> R {
        let _guard = HOME_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let prev = std::env::var_os("HOME");
        // SAFETY: tests serialise through HOME_LOCK so concurrent set_var is impossible.
        unsafe { std::env::set_var("HOME", home) };
        let out = f();
        // SAFETY: same lock guarantees no concurrent reader observing the
        // mutation race here.
        unsafe {
            match prev {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
        }
        out
    }

    #[test]
    fn count_sessions_empty_returns_zero() {
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root = tmp.path().join("project-a");
            std::fs::create_dir_all(&root).unwrap();
            assert_eq!(count_sessions("missing-scope", Some(&root)), 0);
        });
    }

    #[test]
    fn count_sessions_counts_only_json() {
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root = tmp.path().join("project-b");
            std::fs::create_dir_all(&root).unwrap();
            let dir = scope_dir("scope-1", Some(&root));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("a.json"), "{}").unwrap();
            std::fs::write(dir.join("b.json"), "{}").unwrap();
            std::fs::write(dir.join("c.tmp"), "ignored").unwrap();
            std::fs::create_dir_all(dir.join("subdir")).unwrap();
            assert_eq!(count_sessions("scope-1", Some(&root)), 2);
        });
    }

    #[test]
    fn delete_project_data_idempotent() {
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root = tmp.path().join("project-c");
            std::fs::create_dir_all(&root).unwrap();
            // First call: dir doesn't exist yet — must not panic.
            delete_project_data(&root);
            let data = crate::config::data_dir(Some(&root));
            std::fs::create_dir_all(&data).unwrap();
            std::fs::write(data.join("explorations.json"), "[]").unwrap();
            delete_project_data(&root);
            assert!(!data.exists());
            // Second call after deletion: still a no-op.
            delete_project_data(&root);
        });
    }

    #[test]
    fn delete_project_data_removes_chats_and_explorations() {
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root_a = tmp.path().join("project-a");
            let root_b = tmp.path().join("project-b");
            std::fs::create_dir_all(&root_a).unwrap();
            std::fs::create_dir_all(&root_b).unwrap();

            // Seed both projects with chats + explorations.json.
            for root in [&root_a, &root_b] {
                let dir = scope_dir("scope-x", Some(root));
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(dir.join("s.json"), "{}").unwrap();
                let data = crate::config::data_dir(Some(root));
                std::fs::write(data.join("explorations.json"), "[]").unwrap();
            }

            delete_project_data(&root_a);

            // Project A's data dir is gone.
            assert!(!crate::config::data_dir(Some(&root_a)).exists());
            // Project B is untouched.
            let data_b = crate::config::data_dir(Some(&root_b));
            assert!(data_b.join("explorations.json").exists());
            assert!(data_b.join("chats").join("scope-x").join("s.json").exists());
        });
    }
}
