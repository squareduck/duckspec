//! Per-scope chat session model and persistence.
//!
//! A "scope" is a change name, "caps", or "codex". Each scope can have multiple
//! chat sessions, stored under `<data>/chats/<scope>/<session_id>.json`.

use std::path::{Path, PathBuf};

use duckchat::ModelRef;
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
    /// Assistant thinking, distinct from answer prose.
    Reasoning(String),
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
    /// Streaming reasoning, distinct from answer prose. Not persisted as a
    /// field — folded into `ContentBlock::Reasoning` on flush / snapshot.
    pub pending_reasoning: String,
    /// Count of answer-after-thought draft replacements in the current turn
    /// span (reset on tool use / turn end). In-memory only — not persisted.
    pub answer_replace_count: u32,
    /// True after the thrash budget trips; further answer/reasoning deltas
    /// are dropped until the counter is reset. In-memory only.
    pub answer_thrash_tripped: bool,
    /// Agent CLI session id, used to resume the same agent-side conversation
    /// across turns. Set after the first successful turn; persisted so
    /// conversations can be resumed across app restarts.
    pub agent_session_id: Option<String>,
    /// The harness that owns `agent_session_id`. A session id is
    /// harness-specific — a Claude id can't `session/load` under grok and vice
    /// versa — so this records which backend produced it. `None` on sessions
    /// saved before harnesses existed (treated as `claude-code`).
    pub session_harness: Option<String>,
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
    /// Model override for this chat, as a harness-tagged `ModelRef`. `None`
    /// means "use the project default" (which itself may be unset, in which
    /// case the built-in default applies). Persisted so a pinned model survives
    /// resume; a legacy bare-string value loads as the `claude-code` harness
    /// via `ModelRef`'s deserialize shim.
    pub selected_model: Option<ModelRef>,
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
            pending_reasoning: String::new(),
            answer_replace_count: 0,
            answer_thrash_tripped: false,
            agent_session_id: None,
            session_harness: None,
            title: None,
            last_seeded_description: None,
            selected_model: None,
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
    // `alias` keeps sessions written under the old field name loadable.
    #[serde(default, alias = "claude_session_id")]
    agent_session_id: Option<String>,
    #[serde(default)]
    session_harness: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    last_seeded_description: Option<String>,
    #[serde(default)]
    selected_model: Option<ModelRef>,
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

/// Build a title-refresh summarizer target from the session's **current**
/// conversation: all non-priming, non-bare-slash user turns joined in order.
/// Unlike [`title_summarization_target`], later user turns are included so a
/// retitle can track topic drift. Returns `None` when nothing is summarizable.
pub fn title_refresh_target(session: &ChatSession) -> Option<TitleTarget> {
    let mut last_command_source: Option<String> = None;
    let mut parts: Vec<String> = Vec::new();
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
        parts.push(text);
    }
    if parts.is_empty() {
        return None;
    }
    Some(TitleTarget {
        message: parts.join("\n\n"),
        command_hint_source: last_command_source,
    })
}

/// Commit a manual exploration rename. Non-empty trimmed text becomes the new
/// `display_name` and returns `true`. Blank or whitespace-only input leaves
/// the name unchanged and returns `false`.
pub fn rename_exploration(exp: &mut Exploration, name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }
    exp.display_name = trimmed.to_string();
    true
}

/// Apply a title string to a session. When `force` is false, an existing title
/// is left alone (first-turn auto-title). When `force` is true, a non-empty
/// title overwrites. Empty/whitespace titles never write. Returns whether the
/// session title was updated.
pub fn apply_session_title_value(session: &mut ChatSession, title: &str, force: bool) -> bool {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return false;
    }
    if session.title.is_some() && !force {
        return false;
    }
    session.title = Some(trimmed.to_string());
    true
}

/// Map a title-summary oneshot result to an accepted title. Empty strings and
/// errors yield `None` so callers leave existing labels alone.
pub fn accept_title_summary_result(result: &Result<String, String>) -> Option<String> {
    match result {
        Ok(t) => {
            let t = t.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        Err(_) => None,
    }
}

/// True when `text` starts with a `/` followed by a non-empty token. Used
/// to detect command-bearing turns regardless of whether they have content
/// after the command.
fn starts_with_slash_command(text: &str) -> bool {
    let trimmed = text.trim_start();
    let Some(rest) = trimmed.strip_prefix('/') else {
        return false;
    };
    rest.chars().next().is_some_and(|c| !c.is_whitespace())
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
    let label_for =
        |s: &ChatSession| -> String { s.title.clone().unwrap_or_else(|| scope_label.to_string()) };
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
            pending_reasoning: String::new(),
            answer_replace_count: 0,
            answer_thrash_tripped: false,
            agent_session_id: persisted.agent_session_id,
            session_harness: persisted.session_harness,
            title: persisted.title,
            last_seeded_description: persisted.last_seeded_description,
            selected_model: persisted.selected_model,
        });
    }
    sessions.sort_by(|a, b| b.created_at_nanos.cmp(&a.created_at_nanos));
    // At load time we don't yet have the caller's preferred label (exploration
    // display_name may differ from scope key). Use the scope key as a
    // placeholder; callers re-reconcile with the right label afterwards.
    reconcile_display_names(&mut sessions, scope);
    sessions
}

/// Write `data` to `path` atomically: write to a temp file beside the
/// destination, then rename it into place (an atomic replace on the same
/// filesystem — the temp file lives in the destination directory so the
/// rename stays intra-filesystem). The prior contents at `path` remain
/// intact until the rename succeeds, so an interrupted write never
/// truncates them. On any failure the temp file is removed so no `.tmp`
/// residue is left behind.
fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    if let Err(e) = std::fs::write(&tmp, data) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
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
        agent_session_id: session.agent_session_id.clone(),
        session_harness: session.session_harness.clone(),
        title: session.title.clone(),
        last_seeded_description: session.last_seeded_description.clone(),
        selected_model: session.selected_model.clone(),
    };
    let data = serde_json::to_string_pretty(&persisted)?;
    write_atomic(&path, data.as_bytes())?;
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

/// Message count of a persisted session file, or 0 when it is missing or
/// unparseable. Used by `merge_scope` to pick the fuller copy on collision.
fn persisted_message_count(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str::<PersistedSession>(&data).ok())
        .map(|s| s.messages.len())
        .unwrap_or(0)
}

/// Migrate every session file from scope `from` into scope `to` without ever
/// overwriting or discarding one. For each `<id>.json` in the source: move it
/// when the target has no session with that id; on a same-id collision keep the
/// copy with more messages under `<id>.json` and set the loser aside as
/// `<id>.json.orphan` rather than deleting it. The emptied source directory is
/// removed once every session has been moved.
pub fn merge_scope(from: &str, to: &str, project_root: Option<&Path>) {
    let from_dir = scope_dir(from, project_root);
    let to_dir = scope_dir(to, project_root);
    let entries = match std::fs::read_dir(&from_dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    if let Err(e) = std::fs::create_dir_all(&to_dir) {
        tracing::warn!(from, to, "failed to create target scope directory: {e}");
        return;
    }
    for entry in entries.flatten() {
        let src = entry.path();
        if src.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Some(file_name) = src.file_name() else {
            continue;
        };
        let dst = to_dir.join(file_name);
        if !dst.exists() {
            // Target slot free — move the session straight over.
            if let Err(e) = std::fs::rename(&src, &dst) {
                tracing::warn!(from, to, "failed to move session file: {e}");
            }
            continue;
        }
        // Same-id collision: keep the fuller copy, preserve the loser as
        // `<id>.json.orphan` (never delete it).
        let orphan = dst.with_extension("json.orphan");
        if persisted_message_count(&src) > persisted_message_count(&dst) {
            // Source is fuller: set the target copy aside, then move source in.
            if let Err(e) = std::fs::rename(&dst, &orphan) {
                tracing::warn!(from, to, "failed to set aside displaced session: {e}");
                continue;
            }
            if let Err(e) = std::fs::rename(&src, &dst) {
                tracing::warn!(from, to, "failed to move fuller session: {e}");
            }
        } else {
            // Target is fuller (or tied): preserve the source copy instead.
            if let Err(e) = std::fs::rename(&src, &orphan) {
                tracing::warn!(from, to, "failed to set aside source session: {e}");
            }
        }
    }
    // Remove the source directory once emptied. `remove_dir` fails if any
    // residue remains, which is a benign warning, not data loss.
    if let Err(e) = std::fs::remove_dir(&from_dir)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(from, "failed to remove emptied source scope directory: {e}");
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

    // ── exploration list labels (rename + refresh) ──────────────────────

    /// @spec exploration/list-labels Manual rename updates the exploration label: Non-empty rename replaces the list label and persists
    #[test]
    fn rename_exploration_non_empty_replaces_and_persists() {
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root = tmp.path().join("proj-rename");
            std::fs::create_dir_all(&root).unwrap();

            let mut exp = Exploration {
                id: "exploration-1".into(),
                display_name: "Exploration 3".into(),
                idea_path: None,
                session_count: 0,
            };
            assert!(rename_exploration(&mut exp, "Cloud agent options"));
            assert_eq!(exp.display_name, "Cloud agent options");

            save_explorations(std::slice::from_ref(&exp), 1, Some(&root));
            let (loaded, _) = load_explorations(Some(&root));
            let found = loaded.iter().find(|e| e.id == "exploration-1").unwrap();
            assert_eq!(found.display_name, "Cloud agent options");
        });
    }

    /// @spec exploration/list-labels Manual rename updates the exploration label: Blank rename leaves the label unchanged
    #[test]
    fn rename_exploration_blank_leaves_label_unchanged() {
        let mut exp = Exploration {
            id: "exploration-1".into(),
            display_name: "Cloud agent options".into(),
            idea_path: None,
            session_count: 0,
        };
        assert!(!rename_exploration(&mut exp, "   "));
        assert!(!rename_exploration(&mut exp, ""));
        assert_eq!(exp.display_name, "Cloud agent options");
    }

    /// @spec exploration/list-labels Refresh retitles from the active session chat: Refresh input includes later user turns when present
    #[test]
    fn refresh_target_includes_later_user_turns() {
        let s = session_with(vec![
            user_msg("Hello"),
            assistant_msg("hi"),
            user_msg("Focus on rename and retitle in the CHANGE list"),
        ]);
        let t = title_refresh_target(&s).unwrap();
        assert!(t.message.contains("Hello"));
        assert!(t
            .message
            .contains("Focus on rename and retitle in the CHANGE list"));
        // First-turn auto path still only returns the first message.
        let first = title_summarization_target(&s).unwrap();
        assert_eq!(first.message, "Hello");
    }

    /// @spec exploration/list-labels Refresh retitles from the active session chat: Refresh with no summarizable content leaves labels unchanged
    #[test]
    fn refresh_with_no_content_leaves_labels_unchanged() {
        let exp = Exploration {
            id: "exploration-1".into(),
            display_name: "Keep me".into(),
            idea_path: None,
            session_count: 0,
        };
        let mut session = session_with(vec![user_msg("/ds-apply"), priming_msg("AGENTS.md")]);
        session.title = Some("Keep me".into());

        assert!(title_refresh_target(&session).is_none());
        // No target → caller must not apply; labels stay put.
        assert_eq!(exp.display_name, "Keep me");
        assert_eq!(session.title.as_deref(), Some("Keep me"));
    }

    /// @spec exploration/list-labels Refresh retitles from the active session chat: Refresh overwrites an existing title and exploration label
    #[test]
    fn refresh_overwrites_existing_title_and_exploration_label() {
        let mut exp = Exploration {
            id: "exploration-1".into(),
            display_name: "Old title".into(),
            idea_path: None,
            session_count: 0,
        };
        let mut session = session_with(vec![user_msg("talk about new direction")]);
        session.title = Some("Old title".into());

        let summary = "New direction";
        assert!(apply_session_title_value(&mut session, summary, true));
        assert!(rename_exploration(&mut exp, summary));
        assert_eq!(session.title.as_deref(), Some("New direction"));
        assert_eq!(exp.display_name, "New direction");
    }

    /// @spec exploration/list-labels Refresh retitles from the active session chat: Failed or empty refresh leaves labels unchanged
    #[test]
    fn failed_or_empty_refresh_leaves_labels_unchanged() {
        let mut exp = Exploration {
            id: "exploration-1".into(),
            display_name: "Keep me".into(),
            idea_path: None,
            session_count: 0,
        };
        let mut session = session_with(vec![user_msg("content")]);
        session.title = Some("Keep me".into());

        for result in [
            Err("timeout".into()),
            Ok(String::new()),
            Ok("   ".into()),
        ] {
            if let Some(title) = accept_title_summary_result(&result) {
                apply_session_title_value(&mut session, &title, true);
                rename_exploration(&mut exp, &title);
            }
        }
        assert_eq!(exp.display_name, "Keep me");
        assert_eq!(session.title.as_deref(), Some("Keep me"));
    }

    // ── session count / delete project data ─────────────────────────────

    use crate::test_support::{FsTmp, with_home};

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

    /// @spec chat/persistence Atomic session writes: A failed save leaves the prior contents intact
    #[test]
    fn failed_save_leaves_prior_contents_intact() {
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root = tmp.path().join("project-atomic");
            std::fs::create_dir_all(&root).unwrap();

            // GIVEN a session already persisted to disk.
            let mut session = ChatSession::new("atomic-scope".into());
            session.id = "100".into();
            session.messages = vec![user_msg("first")];
            save_session(&session, Some(&root)).unwrap();

            // Block the temp path so the next write fails partway: a directory
            // sits where `write_atomic` needs to place `<id>.json.tmp`, so the
            // temp write errors before the destination is ever renamed.
            let dir = scope_dir("atomic-scope", Some(&root));
            std::fs::create_dir_all(dir.join("100.json.tmp")).unwrap();

            // WHEN a subsequent save of that session fails partway through.
            session.messages = vec![user_msg("first"), user_msg("second")];
            assert!(save_session(&session, Some(&root)).is_err());

            // THEN the file on disk still parses as the previously-persisted
            // session (one message, not two).
            let reloaded = load_sessions_for("atomic-scope", Some(&root));
            assert_eq!(reloaded.len(), 1);
            assert_eq!(reloaded[0].messages.len(), 1);
        });
    }

    /// @spec chat/persistence Non-destructive scope migration: Migration into an occupied scope keeps both scopes' sessions
    #[test]
    fn migration_into_occupied_scope_keeps_both_sessions() {
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root = tmp.path().join("project-merge");
            std::fs::create_dir_all(&root).unwrap();

            // GIVEN a source scope holding a session, AND a target scope
            // already holding a different session.
            let mut src = ChatSession::new("src-scope".into());
            src.id = "1".into();
            src.messages = vec![user_msg("from source")];
            save_session(&src, Some(&root)).unwrap();

            let mut tgt = ChatSession::new("dst-scope".into());
            tgt.id = "2".into();
            tgt.messages = vec![user_msg("from target")];
            save_session(&tgt, Some(&root)).unwrap();

            // WHEN the source scope is migrated into the target scope.
            merge_scope("src-scope", "dst-scope", Some(&root));

            // THEN the target scope afterward holds both sessions.
            let sessions = load_sessions_for("dst-scope", Some(&root));
            assert_eq!(sessions.len(), 2);
            assert!(sessions.iter().any(|s| s.id == "1"));
            assert!(sessions.iter().any(|s| s.id == "2"));
        });
    }

    /// @spec chat/persistence Non-destructive scope migration: Same-id collision keeps the fuller session and preserves the other
    #[test]
    fn same_id_collision_keeps_fuller_and_preserves_other() {
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root = tmp.path().join("project-collision");
            std::fs::create_dir_all(&root).unwrap();

            // GIVEN a source and target scope each holding a session with the
            // same id, AND the source copy has more messages than the target.
            let mut src = ChatSession::new("src-scope".into());
            src.id = "7".into();
            src.messages = vec![user_msg("a"), user_msg("b"), user_msg("c")];
            save_session(&src, Some(&root)).unwrap();

            let mut tgt = ChatSession::new("dst-scope".into());
            tgt.id = "7".into();
            tgt.messages = vec![user_msg("a")];
            save_session(&tgt, Some(&root)).unwrap();

            // WHEN the source scope is migrated into the target scope.
            merge_scope("src-scope", "dst-scope", Some(&root));

            // THEN the target's session for that id has the fuller set of
            // messages...
            let sessions = load_sessions_for("dst-scope", Some(&root));
            let kept = sessions.iter().find(|s| s.id == "7").unwrap();
            assert_eq!(kept.messages.len(), 3);
            // ...AND the displaced copy is preserved rather than deleted.
            let orphan = scope_dir("dst-scope", Some(&root)).join("7.json.orphan");
            assert!(orphan.exists());
        });
    }

    /// @spec chat/persistence In-flight turn durability: An in-flight turn survives a promotion
    #[test]
    fn in_flight_turn_survives_promotion() {
        use crate::area::change::{State, promote_exploration};
        use crate::area::interaction::{AgentSession, InteractionState};
        use crate::scope::{Scope, ScopeKind};
        use std::collections::HashMap;

        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root = tmp.path().join("project-promote");
            std::fs::create_dir_all(&root).unwrap();

            let mut state = State::new(Some(&root));
            let exp_id = "exploration-1".to_string();
            state.explorations.push(Exploration {
                id: exp_id.clone(),
                display_name: "Exp".into(),
                idea_path: None,
                session_count: 0,
            });

            // GIVEN a session with messages streamed since its last persist:
            // the session lives only in memory (never saved to disk).
            let mut ax = AgentSession::new(exp_id.clone(), ScopeKind::Exploration);
            ax.session.id = "sess-1".into();
            ax.session.messages = vec![user_msg("streamed one"), user_msg("streamed two")];
            ax.needs_flush = true;
            ax.session.is_streaming = true;
            let mut ix = InteractionState::default();
            ix.sessions.push(ax);
            let mut interactions: HashMap<Scope, InteractionState> = HashMap::new();
            interactions.insert(Scope::Exploration(exp_id.clone()), ix);

            // WHEN the scope's in-memory state is migrated by a promotion.
            promote_exploration(
                &mut state,
                &mut interactions,
                &exp_id,
                "real-change",
                Some(&root),
            );

            // THEN the persisted session under the new scope includes those
            // streamed messages.
            let persisted = load_sessions_for("real-change", Some(&root));
            let sess = persisted.iter().find(|s| s.id == "sess-1").unwrap();
            assert_eq!(sess.messages.len(), 2);
        });
    }

    /// @spec chat/persistence In-flight turn durability: Streamed messages are persisted before turn completion
    #[test]
    fn eager_flush_persists_streamed_messages_before_turn_completion() {
        use crate::area::interaction::{AgentSession, InteractionState, flush_dirty_sessions};
        use crate::scope::ScopeKind;

        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root = tmp.path().join("project-eager");
            std::fs::create_dir_all(&root).unwrap();

            // GIVEN a turn that has streamed messages and has not yet completed.
            let mut ax = AgentSession::new("eager-scope".into(), ScopeKind::Change);
            ax.session.id = "sess-eager".into();
            ax.session.messages = vec![user_msg("streamed so far")];
            ax.session.is_streaming = true;
            ax.needs_flush = true;
            let mut ix = InteractionState::default();
            ix.sessions.push(ax);

            // WHEN an eager flush occurs.
            flush_dirty_sessions(&mut ix, Some(&root));

            // THEN the persisted session includes the messages streamed so far.
            let persisted = load_sessions_for("eager-scope", Some(&root));
            let sess = persisted.iter().find(|s| s.id == "sess-eager").unwrap();
            assert_eq!(sess.messages.len(), 1);
        });
    }

    /// @spec chat/persistence Reasoning content: Reasoning content round-trips through persist and load
    #[test]
    fn reasoning_content_round_trips_through_persist_and_load() {
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root = tmp.path().join("project-reasoning-rt");
            std::fs::create_dir_all(&root).unwrap();

            // GIVEN a session whose messages include a Reasoning content block.
            let mut session = ChatSession::new("reasoning-scope".into());
            session.id = "sess-reasoning".into();
            session.messages = vec![ChatMessage {
                role: Role::Assistant,
                content: vec![ContentBlock::Reasoning("consider the edge cases".into())],
                timestamp: String::new(),
                is_priming: false,
            }];

            // WHEN the session is persisted and loaded again.
            save_session(&session, Some(&root)).unwrap();
            let loaded = load_sessions_for("reasoning-scope", Some(&root));

            // THEN the loaded session includes a Reasoning block with the same body.
            let sess = loaded.iter().find(|s| s.id == "sess-reasoning").unwrap();
            assert_eq!(sess.messages.len(), 1);
            match &sess.messages[0].content[..] {
                [ContentBlock::Reasoning(body)] => {
                    assert_eq!(body, "consider the edge cases");
                }
                other => panic!("expected single Reasoning block, got {other:?}"),
            }
        });
    }

    /// @spec chat/persistence Reasoning content: A legacy session without Reasoning still loads
    #[test]
    fn legacy_session_without_reasoning_still_loads() {
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root = tmp.path().join("project-legacy");
            std::fs::create_dir_all(&root).unwrap();

            // GIVEN a session file whose messages use only Text, ToolUse, and
            // ToolResult content (no Reasoning variant).
            let mut session = ChatSession::new("legacy-scope".into());
            session.id = "sess-legacy".into();
            session.messages = vec![
                user_msg("hello"),
                ChatMessage {
                    role: Role::Assistant,
                    content: vec![
                        ContentBlock::Text("hi".into()),
                        ContentBlock::ToolUse {
                            id: "t1".into(),
                            name: "Read".into(),
                            input: "{\"path\":\"a\"}".into(),
                        },
                        ContentBlock::ToolResult {
                            id: "t1".into(),
                            name: "Read".into(),
                            output: "file contents".into(),
                        },
                    ],
                    timestamp: String::new(),
                    is_priming: false,
                },
            ];
            save_session(&session, Some(&root)).unwrap();

            // Confirm the on-disk JSON has no Reasoning variant.
            let path = scope_dir("legacy-scope", Some(&root)).join("sess-legacy.json");
            let raw = std::fs::read_to_string(&path).unwrap();
            assert!(!raw.contains("Reasoning"), "fixture must not contain Reasoning");

            // WHEN the session is loaded.
            let loaded = load_sessions_for("legacy-scope", Some(&root));

            // THEN the load succeeds AND the loaded messages match the file's content.
            let sess = loaded.iter().find(|s| s.id == "sess-legacy").unwrap();
            assert_eq!(sess.messages.len(), 2);
            assert!(matches!(
                &sess.messages[0].content[..],
                [ContentBlock::Text(t)] if t == "hello"
            ));
            assert_eq!(sess.messages[1].content.len(), 3);
            assert!(matches!(
                &sess.messages[1].content[0],
                ContentBlock::Text(t) if t == "hi"
            ));
            assert!(matches!(
                &sess.messages[1].content[1],
                ContentBlock::ToolUse { id, name, .. } if id == "t1" && name == "Read"
            ));
            assert!(matches!(
                &sess.messages[1].content[2],
                ContentBlock::ToolResult { id, name, .. } if id == "t1" && name == "Read"
            ));
        });
    }

    /// @spec chat/persistence In-flight turn durability: Eager flush includes pending reasoning as Reasoning content
    #[test]
    fn eager_flush_includes_pending_reasoning_as_reasoning_content() {
        use crate::area::interaction::{AgentSession, InteractionState, flush_dirty_sessions};
        use crate::scope::ScopeKind;

        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root = tmp.path().join("project-eager-reasoning");
            std::fs::create_dir_all(&root).unwrap();

            // GIVEN a turn that has streamed reasoning into the pending
            // reasoning buffer and has not yet completed.
            let mut ax = AgentSession::new("eager-reasoning-scope".into(), ScopeKind::Change);
            ax.session.id = "sess-eager-r".into();
            ax.session.pending_reasoning = "thinking out loud".into();
            ax.session.is_streaming = true;
            ax.needs_flush = true;
            let mut ix = InteractionState::default();
            ix.sessions.push(ax);

            // WHEN an eager flush occurs.
            flush_dirty_sessions(&mut ix, Some(&root));

            // THEN the persisted session includes that reasoning as Reasoning
            // content AND that body is not stored as Text content.
            let persisted = load_sessions_for("eager-reasoning-scope", Some(&root));
            let sess = persisted.iter().find(|s| s.id == "sess-eager-r").unwrap();
            assert_eq!(sess.messages.len(), 1);
            match &sess.messages[0].content[..] {
                [ContentBlock::Reasoning(body)] => {
                    assert_eq!(body, "thinking out loud");
                }
                other => panic!("expected Reasoning content, got {other:?}"),
            }
            // In-memory session is left untouched (snapshot only).
            assert_eq!(
                ix.sessions[0].session.pending_reasoning,
                "thinking out loud"
            );
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
