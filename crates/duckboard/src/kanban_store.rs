//! Kanban card model and persistence.
//!
//! Cards live in `<data>/kanban.json` per project. They are a planning
//! layer on top of duckspec concepts: each card optionally attaches an
//! Exploration and/or a Change, and the kanban column is derived from
//! those attachments plus their on-disk state.

use std::path::Path;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// A planning card. The kanban column is derived from attachments, not
/// stored — see `area::kanban::classify`. The title shown in the board
/// is derived from the first line of `description` (see
/// `area::kanban::derive_title`) — there is no separate title field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub created_at_nanos: i128,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub exploration_id: Option<String>,
    #[serde(default)]
    pub change_name: Option<String>,
    /// Set when the user manually discards a card that never made it to
    /// completion. Renders in the Archived column with a "discarded" mark.
    #[serde(default)]
    pub archived_at_nanos: Option<i128>,
}

impl Card {
    pub fn new() -> Self {
        let nanos = OffsetDateTime::now_local()
            .unwrap_or_else(|_| OffsetDateTime::now_utc())
            .unix_timestamp_nanos();
        Self {
            id: format!("card-{nanos}"),
            created_at_nanos: nanos,
            description: String::new(),
            exploration_id: None,
            change_name: None,
            archived_at_nanos: None,
        }
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct KanbanData {
    cards: Vec<Card>,
}

pub fn load(project_root: Option<&Path>) -> Vec<Card> {
    let path = crate::config::data_dir(project_root).join("kanban.json");
    let Ok(data) = std::fs::read_to_string(&path) else {
        return vec![];
    };
    match serde_json::from_str::<KanbanData>(&data) {
        Ok(state) => state.cards,
        Err(e) => {
            tracing::warn!("failed to parse kanban.json: {e}");
            vec![]
        }
    }
}

pub fn save(cards: &[Card], project_root: Option<&Path>) {
    let dir = crate::config::data_dir(project_root);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::warn!("failed to create kanban data directory: {e}");
        return;
    }
    match serde_json::to_string_pretty(&KanbanData {
        cards: cards.to_vec(),
    }) {
        Ok(data) => {
            if let Err(e) = std::fs::write(dir.join("kanban.json"), data) {
                tracing::warn!("failed to write kanban.json: {e}");
            }
        }
        Err(e) => tracing::warn!("failed to serialize kanban: {e}"),
    }
}
