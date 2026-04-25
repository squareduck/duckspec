//! Codex area — browse codex entries.

use std::collections::HashSet;

use iced::widget::{column, container, text};
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
    pub list_scroll: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            expanded_nodes: HashSet::new(),
            section_expanded: true,
            tabs: tab_bar::TabState::default(),
            interaction: InteractionState::default(),
            list_scroll: 0.0,
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
    ScrollList(f32),
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
                    interaction::clear_single_session(
                        &mut state.interaction,
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
                        &mut state.interaction,
                        other,
                        "codex",
                        "codex",
                        crate::scope::ScopeKind::Codex,
                        project.project_root.as_deref(),
                        highlighter,
                    );
                }
            }
        }
        Message::TabContent(tab_bar::TabContentMsg::EditorAction(_)) => {
            // Intercepted by `main::update` for `Message::Codex` so the async
            // highlight `Task` can be propagated. No-op defensive fallback.
            let _ = highlighter;
        }
        Message::TabContent(tab_bar::TabContentMsg::OpenInNewTab(_)) => {
            // Diff tabs only surface in the change area; ignore elsewhere.
        }
        Message::TabContent(tab_bar::TabContentMsg::SearchSliceAction(idx, action)) => {
            crate::handle_search_slice_action(&mut state.tabs, idx, action);
        }
        Message::TabContent(tab_bar::TabContentMsg::OpenSearchSlice(idx)) => {
            crate::handle_open_search_slice(&mut state.tabs, idx, highlighter);
        }
        Message::ScrollList(offset) => {
            state.list_scroll = offset;
        }
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    interaction::area_layout(
        view_list(state, project),
        view_content(state),
        &state.interaction,
        SessionControls::Single,
        Message::Interaction,
    )
}

fn view_list<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
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
            state.tabs.active_tab().map(|t| t.id.as_str()),
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

fn view_content<'a>(state: &'a State) -> Element<'a, Message> {
    let bar = tab_bar::view_bar(&state.tabs, Message::SelectTab, Message::CloseTab);
    let body = tab_bar::view_content(&state.tabs).map(Message::TabContent);

    column![bar, body].height(Length::Fill).into()
}

fn open_artifact(
    state: &mut State,
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
            &mut state.tabs,
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

pub fn breadcrumbs(state: &State) -> Vec<String> {
    let mut crumbs = vec!["Codex".into()];
    if let Some(tab) = state.tabs.active_tab() {
        let rest = tab.id.strip_prefix("codex/").unwrap_or(&tab.id);
        crumbs.extend(rest.split('/').map(String::from));
    }
    crumbs
}
