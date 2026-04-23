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
}

/// Return `(first user text, first assistant text)` for a session, or `None`
/// if either half is missing. Used by the title summariser after the first
/// successful turn.
pub fn first_exchange(session: &ChatSession) -> Option<(String, String)> {
    let mut user: Option<String> = None;
    let mut assistant: Option<String> = None;
    for msg in &session.messages {
        for block in &msg.content {
            if let ContentBlock::Text(t) = block {
                match msg.role {
                    Role::User if user.is_none() => {
                        user = Some(t.clone());
                    }
                    Role::Assistant if user.is_some() && assistant.is_none() => {
                        assistant = Some(t.clone());
                    }
                    _ => {}
                }
            }
        }
        if user.is_some() && assistant.is_some() {
            break;
        }
    }
    match (user, assistant) {
        (Some(u), Some(a)) => Some((u, a)),
        _ => None,
    }
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
/// summariser without moving the chat directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exploration {
    pub id: String,
    pub display_name: String,
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
    (state.explorations, state.counter)
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
