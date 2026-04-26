//! Codex area — browse codex entries.

use std::collections::HashSet;

use iced::widget::{column, container, text};
use iced::Element;

use crate::data::ProjectData;
use crate::theme;
use crate::widget::{collapsible, tab_bar, tree_view, vertical_scroll};

use super::interaction::{self, InteractionState};

// ── State ────────────────────────────────────────────────────────────────────

pub struct State {
    pub expanded_nodes: HashSet<String>,
    pub section_expanded: bool,
    pub list_scroll: f32,
    /// Artifact id of the currently selected codex entry. Drives list-row
    /// highlighting independently of which tab is currently active, so the
    /// selection survives switching to a file tab or another area.
    pub selected: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            expanded_nodes: HashSet::new(),
            section_expanded: true,
            list_scroll: 0.0,
            selected: None,
        }
    }
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    ToggleSection,
    ToggleNode(String),
    SelectItem(String),
    Interaction(interaction::Msg),
    ScrollList(f32),
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(
    state: &mut State,
    tabs: &mut tab_bar::TabState,
    interaction_state: &mut InteractionState,
    message: Message,
    project: &ProjectData,
    highlighter: &crate::highlight::SyntaxHighlighter,
) {
    match message {
        Message::ToggleSection => {
            state.section_expanded = !state.section_expanded;
        }
        Message::ToggleNode(id) => {
            if !state.expanded_nodes.remove(&id) {
                state.expanded_nodes.insert(id);
            }
        }
        Message::SelectItem(id) => {
            state.selected = Some(id.clone());
            open_artifact(tabs, &id, project, highlighter);
        }
        Message::Interaction(msg) => match msg {
            interaction::Msg::ClearSession => {
                interaction::clear_single_session(
                    interaction_state,
                    "codex",
                    "codex",
                    crate::scope::ScopeKind::Codex,
                    project.project_root.as_deref(),
                );
            }
            interaction::Msg::NewSession | interaction::Msg::SelectSession(_) => {
                // Codex is single-session; ignore.
            }
            other => {
                interaction::update_with_side_effects(
                    interaction_state,
                    other,
                    "codex",
                    "codex",
                    crate::scope::ScopeKind::Codex,
                    project.project_root.as_deref(),
                    highlighter,
                );
            }
        },
        Message::ScrollList(offset) => {
            state.list_scroll = offset;
        }
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view_list<'a>(
    state: &'a State,
    project: &'a ProjectData,
    _tabs: &'a tab_bar::TabState,
) -> Element<'a, Message> {
    let tree = if project.codex_entries.is_empty() {
        container(
            text("No codex entries")
                .size(theme::font_md())
                .color(theme::text_muted()),
        )
        .padding([theme::SPACING_XS, theme::SPACING_SM])
        .into()
    } else {
        tree_view::view(
            &project.codex_entries,
            &state.expanded_nodes,
            state.selected.as_deref(),
            &HashSet::new(),
            Message::ToggleNode,
            Message::SelectItem,
        )
    };

    let section = collapsible::view(
        "Codex",
        state.section_expanded,
        Message::ToggleSection,
        tree,
    );

    vertical_scroll::view(
        state.list_scroll,
        Message::ScrollList,
        column![section].spacing(0.0),
    )
}

fn open_artifact(
    tabs: &mut tab_bar::TabState,
    id: &str,
    project: &ProjectData,
    highlighter: &crate::highlight::SyntaxHighlighter,
) {
    if let Some(content) = project.read_artifact(id) {
        let title = id
            .rsplit('/')
            .next()
            .unwrap_or(id)
            .trim_end_matches(".md")
            .to_string();
        let path = project.duckspec_root.as_ref().map(|r| r.join(id));
        crate::open_artifact_tab(
            tabs,
            id.to_string(),
            title,
            content,
            id,
            path,
            highlighter,
        );
    }
}

// ── Breadcrumbs ──────────────────────────────────────────────────────────────

pub fn breadcrumbs(tabs: &tab_bar::TabState) -> Vec<String> {
    let mut crumbs = vec!["Codex".into()];
    if let Some(tab) = tabs.active_tab() {
        let rest = tab.id.strip_prefix("codex/").unwrap_or(&tab.id);
        crumbs.extend(rest.split('/').map(String::from));
    }
    crumbs
}
