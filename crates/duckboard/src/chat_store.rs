//! Per-change chat session model and XDG-compliant persistence.

use std::path::PathBuf;

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

/// XDG data directory for duckboard chat sessions.
fn chat_dir() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "duckspec", "duckboard")?;
    Some(dirs.data_dir().join("chats"))
}

fn session_path(change_name: &str) -> Option<PathBuf> {
    Some(chat_dir()?.join(format!("{change_name}.json")))
}

/// Load a persisted chat session for the given change.
pub fn load_session(change_name: &str) -> Option<ChatSession> {
    let path = session_path(change_name)?;
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
pub fn save_session(session: &ChatSession) -> anyhow::Result<()> {
    let dir = chat_dir().ok_or_else(|| anyhow::anyhow!("no XDG data directory"))?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", session.change_name));
    let data = serde_json::to_string_pretty(&session.messages)?;
    std::fs::write(path, data)?;
    Ok(())
}
