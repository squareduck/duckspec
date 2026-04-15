//! Per-change chat session model and XDG-compliant persistence.
//!
//! Sessions are stored per-project under the XDG data directory, using a hash
//! of the project root path to isolate different projects.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

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

/// Per-change chat session state.
#[derive(Debug, Clone)]
pub struct ChatSession {
    pub change_name: String,
    pub messages: Vec<ChatMessage>,
    pub is_streaming: bool,
    pub pending_text: String,
}

impl ChatSession {
    pub fn new(change_name: String) -> Self {
        Self {
            change_name,
            messages: Vec::new(),
            is_streaming: false,
            pending_text: String::new(),
        }
    }
}

// ── Persistence ─────────────────────────────────────────────────────────────

/// Derive a short hex hash from a project root path for per-project isolation.
fn project_hash(project_root: &Path) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    project_root.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// XDG data directory for duckboard, scoped to a project when available.
fn data_dir(project_root: Option<&Path>) -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "duckspec", "duckboard")?;
    let base = dirs.data_dir().to_path_buf();
    match project_root {
        Some(root) => Some(base.join("projects").join(project_hash(root))),
        None => Some(base),
    }
}

fn chat_dir(project_root: Option<&Path>) -> Option<PathBuf> {
    Some(data_dir(project_root)?.join("chats"))
}

fn session_path(change_name: &str, project_root: Option<&Path>) -> Option<PathBuf> {
    Some(chat_dir(project_root)?.join(format!("{change_name}.json")))
}

/// Load a persisted chat session for the given change.
pub fn load_session(change_name: &str, project_root: Option<&Path>) -> Option<ChatSession> {
    let path = session_path(change_name, project_root)?;
    let data = std::fs::read_to_string(&path).ok()?;
    let messages: Vec<ChatMessage> = serde_json::from_str(&data).ok()?;
    Some(ChatSession {
        change_name: change_name.to_string(),
        messages,
        is_streaming: false,
        pending_text: String::new(),
    })
}

/// Save a chat session to disk.
pub fn save_session(session: &ChatSession, project_root: Option<&Path>) -> anyhow::Result<()> {
    let dir = chat_dir(project_root).ok_or_else(|| anyhow::anyhow!("no XDG data directory"))?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", session.change_name));
    let data = serde_json::to_string_pretty(&session.messages)?;
    std::fs::write(path, data)?;
    Ok(())
}

/// Delete a persisted chat session from disk.
pub fn delete_session(change_name: &str, project_root: Option<&Path>) {
    if let Some(path) = session_path(change_name, project_root) {
        let _ = std::fs::remove_file(path);
    }
}

/// Rename a persisted session file from one change name to another.
/// Also updates the in-memory session's `change_name`.
pub fn rename_session(session: &mut ChatSession, new_name: &str, project_root: Option<&Path>) {
    let old_name = session.change_name.clone();
    session.change_name = new_name.to_string();

    // Move the persisted file if it exists.
    if let (Some(old_path), Some(new_path)) =
        (session_path(&old_name, project_root), session_path(new_name, project_root))
        && old_path.exists()
    {
        let _ = std::fs::rename(&old_path, &new_path);
    }
}

/// Persisted exploration state.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExplorationData {
    explorations: Vec<String>,
    counter: usize,
}

/// Load persisted exploration list and counter.
pub fn load_explorations(project_root: Option<&Path>) -> (Vec<String>, usize) {
    let Some(path) = data_dir(project_root).map(|d| d.join("explorations.json")) else {
        return (vec![], 0);
    };
    let Ok(data) = std::fs::read_to_string(&path) else {
        return (vec![], 0);
    };
    let Ok(state) = serde_json::from_str::<ExplorationData>(&data) else {
        return (vec![], 0);
    };
    (state.explorations, state.counter)
}

/// Save exploration list and counter to disk.
pub fn save_explorations(explorations: &[String], counter: usize, project_root: Option<&Path>) {
    let Some(dir) = data_dir(project_root) else { return };
    let _ = std::fs::create_dir_all(&dir);
    let state = ExplorationData {
        explorations: explorations.to_vec(),
        counter,
    };
    if let Ok(data) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(dir.join("explorations.json"), data);
    }
}
