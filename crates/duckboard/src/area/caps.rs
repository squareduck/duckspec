//! Caps area — browse the capability tree.

use std::collections::HashSet;

use iced::widget::{column, text};
use iced::{Element, Length};

use crate::data::ProjectData;
use crate::theme;
use crate::widget::{collapsible, tab_bar, tree_view, vertical_scroll};

use super::interaction::{self, InteractionState, SessionControls};

// ── State ────────────────────────────────────────────────────────────────────

pub struct State {
    pub expanded_nodes: HashSet<String>,
    pub section_expanded: bool,
    pub tabs: tab_bar::TabState,
    pub interaction: InteractionState,
}

impl Default for State {
    fn default() -> Self {
        Self {
            expanded_nodes: HashSet::new(),
            section_expanded: true,
            tabs: tab_bar::TabState::default(),
            interaction: InteractionState::default(),
        }
    }
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    ToggleSection,
    ToggleNode(String),
    SelectItem(String),
    SelectTab(usize),
    CloseTab(usize),
    Interaction(interaction::Msg),
    TabContent(tab_bar::TabContentMsg),
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(
    state: &mut State,
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
            open_artifact(state, &id, project, highlighter);
        }
        Message::SelectTab(idx) => state.tabs.select(idx),
        Message::CloseTab(idx) => state.tabs.close(idx),
        Message::Interaction(msg) => {
            match msg {
                interaction::Msg::ClearSession => {
                    interaction::clear_single_session(&mut state.interaction, "caps", project.project_root.as_deref());
                }
                interaction::Msg::NewSession | interaction::Msg::SelectSession(_) => {
                    // Caps is single-session; ignore.
                }
                other => {
                    interaction::update_with_side_effects(
                        &mut state.interaction, other, "caps",
                        project.project_root.as_deref(), highlighter,
                    );
                }
            }
        }
        Message::TabContent(tab_bar::TabContentMsg::EditorAction(action)) => {
            crate::handle_editor_action(&mut state.tabs, action, highlighter);
        }
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(
    state: &'a State,
    project: &'a ProjectData,
) -> Element<'a, Message> {
    interaction::area_layout(
        view_list(state, project),
        view_content(state),
        &state.interaction,
        SessionControls::Single,
        Message::Interaction,
    )
}

fn view_list<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let tree = if project.cap_tree.is_empty() {
        column![
            text("No capabilities found")
                .size(theme::font_md())
                .color(theme::text_muted())
        ]
        .into()
    } else {
        tree_view::view(
            &project.cap_tree,
            &state.expanded_nodes,
            state.tabs.active_tab().map(|t| t.id.as_str()),
            &std::collections::HashSet::new(),
            Message::ToggleNode,
            Message::SelectItem,
        )
    };

    let section = collapsible::view(
        "Capabilities",
        state.section_expanded,
        Message::ToggleSection,
        tree,
    );

    vertical_scroll::view(column![section].spacing(0.0))
}

fn view_content<'a>(state: &'a State) -> Element<'a, Message> {
    let bar = tab_bar::view_bar(
        &state.tabs,
        Message::SelectTab,
        Message::CloseTab,
    );
    let body = tab_bar::view_content(&state.tabs).map(Message::TabContent);

    column![bar, body].height(Length::Fill).into()
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn open_artifact(
    state: &mut State,
    id: &str,
    project: &ProjectData,
    highlighter: &crate::highlight::SyntaxHighlighter,
) {
    if let Some(content) = project.read_artifact(id) {
        let title = id.rsplit('/').next().unwrap_or(id).to_string();
        crate::open_artifact_tab(&mut state.tabs, id.to_string(), title, content, id, highlighter);
    }
}

// ── Breadcrumbs ──────────────────────────────────────────────────────────────

pub fn breadcrumbs(state: &State) -> Vec<String> {
    let mut crumbs = vec!["Capabilities".into()];
    if let Some(tab) = state.tabs.active_tab() {
        let rest = tab.id.strip_prefix("caps/").unwrap_or(&tab.id);
        crumbs.extend(rest.split('/').map(String::from));
    }
    crumbs
}
