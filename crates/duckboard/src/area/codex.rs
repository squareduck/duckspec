//! Codex area — browse codex entries.

use iced::widget::{button, column, row, scrollable, svg, text};
use iced::widget::text::Wrapping;
use iced::{Element, Length};

use crate::data::ProjectData;
use crate::theme;
use crate::widget::{collapsible, tab_bar};

use super::interaction::{self, InteractionState, SessionControls};

const ICON_FILE: &[u8] = include_bytes!("../../assets/icon_file.svg");
const ICON_SIZE: f32 = 14.0;

// ── State ────────────────────────────────────────────────────────────────────

pub struct State {
    pub section_expanded: bool,
    pub tabs: tab_bar::TabState,
    pub interaction: InteractionState,
}

impl Default for State {
    fn default() -> Self {
        Self {
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
    let mut items = column![].spacing(theme::SPACING_XS);

    if project.codex_entries.is_empty() {
        items = items.push(text("No codex entries").size(theme::font_md()).color(theme::text_muted()));
    } else {
        for entry in &project.codex_entries {
            let is_active = state
                .tabs
                .active_tab()
                .is_some_and(|t| t.id == entry.id);
            let style = if is_active {
                theme::list_item_active
                    as fn(&iced::Theme, button::Status) -> button::Style
            } else {
                theme::list_item
            };
            let icon = svg(svg::Handle::from_memory(ICON_FILE))
                .width(ICON_SIZE)
                .height(ICON_SIZE)
                .style(theme::svg_tint(theme::text_muted()));
            let label = row![icon, text(&entry.label).size(theme::font_md()).wrapping(Wrapping::None)]
                .spacing(theme::SPACING_XS)
                .align_y(iced::Center);
            items = items.push(
                button(label)
                    .on_press(Message::SelectItem(entry.id.clone()))
                    .width(Length::Fill)
                    .padding([2.0, theme::SPACING_SM])
                    .style(style),
            );
        }
    }

    let section = collapsible::view(
        "Codex",
        state.section_expanded,
        Message::ToggleSection,
        items.into(),
    );

    scrollable(column![section].spacing(0.0))
        .direction(theme::thin_scrollbar_direction())
        .style(theme::thin_scrollbar)
        .height(Length::Fill)
        .into()
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

