//! Codex area — browse codex entries.

use iced::widget::column;
use iced::{Element, Length};

use crate::data::ProjectData;
use crate::widget::list_view::ListRow;
use crate::widget::{collapsible, list_view, tab_bar, vertical_scroll};

use super::interaction::{self, InteractionState, SessionControls};

const ICON_FILE: &[u8] = include_bytes!("../../assets/icon_file.svg");

// ── State ────────────────────────────────────────────────────────────────────

pub struct State {
    pub section_expanded: bool,
    pub tabs: tab_bar::TabState,
    pub interaction: InteractionState,
    pub list_scroll: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
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
        Message::SelectItem(id) => {
            open_artifact(state, &id, project, highlighter);
        }
        Message::SelectTab(idx) => state.tabs.select(idx),
        Message::CloseTab(idx) => state.tabs.close(idx),
        Message::Interaction(msg) => {
            match msg {
                interaction::Msg::ClearSession => {
                    interaction::clear_single_session(&mut state.interaction, "codex", project.project_root.as_deref());
                }
                interaction::Msg::NewSession | interaction::Msg::SelectSession(_) => {
                    // Codex is single-session; ignore.
                }
                other => {
                    interaction::update_with_side_effects(
                        &mut state.interaction, other, "codex",
                        project.project_root.as_deref(), highlighter,
                    );
                }
            }
        }
        Message::TabContent(tab_bar::TabContentMsg::EditorAction(action)) => {
            crate::handle_editor_action(&mut state.tabs, action, highlighter);
        }
        Message::ScrollList(offset) => {
            state.list_scroll = offset;
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
    let active_id = state.tabs.active_tab().map(|t| t.id.as_str());
    let rows: Vec<ListRow<'a, Message>> = project
        .codex_entries
        .iter()
        .map(|entry| {
            ListRow::new(entry.label.as_str())
                .icon(ICON_FILE)
                .selected(active_id == Some(entry.id.as_str()))
                .on_press(Message::SelectItem(entry.id.clone()))
        })
        .collect();

    let section = collapsible::view(
        "Codex",
        state.section_expanded,
        Message::ToggleSection,
        list_view::view(rows, Some("No codex entries")),
    );

    vertical_scroll::view(state.list_scroll, Message::ScrollList, column![section].spacing(0.0))
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
        crate::open_artifact_tab(&mut state.tabs, id.to_string(), title, content, id, highlighter);
    }
}

// ── Breadcrumbs ──────────────────────────────────────────────────────────────

pub fn breadcrumbs(state: &State, project: &ProjectData) -> Vec<String> {
    let Some(tab) = state.tabs.active_tab() else {
        return vec!["Codex".into()];
    };
    let label = project
        .codex_entries
        .iter()
        .find(|e| e.id == tab.id)
        .map(|e| e.label.clone())
        .unwrap_or_else(|| tab.id.clone());
    vec!["Codex".into(), label]
}

