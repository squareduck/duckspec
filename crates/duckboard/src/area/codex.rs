//! Codex area — browse codex entries.

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Element, Length};

use crate::data::ProjectData;
use crate::theme;
use crate::widget::{interaction_toggle, tab_bar};

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct State {
    pub tabs: tab_bar::TabState,
    pub interaction_visible: bool,
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    SelectItem(String),
    SelectTab(usize),
    CloseTab(usize),
    TogglePin(usize),
    ToggleInteraction,
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(state: &mut State, message: Message, project: &ProjectData) {
    match message {
        Message::SelectItem(id) => {
            if let Some(content) = project.read_artifact(&id) {
                let title = id
                    .rsplit('/')
                    .next()
                    .unwrap_or(&id)
                    .trim_end_matches(".md")
                    .to_string();
                state.tabs.open(id, title, content);
            }
        }
        Message::SelectTab(idx) => state.tabs.select(idx),
        Message::CloseTab(idx) => state.tabs.close(idx),
        Message::TogglePin(idx) => state.tabs.toggle_pin(idx),
        Message::ToggleInteraction => {
            state.interaction_visible = !state.interaction_visible;
        }
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let list = view_list(state, project);
    let content = view_content(state);
    let divider = container(Space::new().height(Length::Fill))
        .width(1.0)
        .style(theme::divider);

    let toggle =
        interaction_toggle::view(state.interaction_visible, Message::ToggleInteraction);

    let mut main_row = row![
        container(list)
            .width(theme::LIST_COLUMN_WIDTH)
            .height(Length::Fill)
            .style(theme::surface),
        divider,
        container(content)
            .width(Length::Fill)
            .height(Length::Fill),
        toggle,
    ];

    if state.interaction_visible {
        main_row = main_row.push(
            container(view_interaction())
                .width(theme::INTERACTION_COLUMN_WIDTH)
                .height(Length::Fill)
                .style(theme::surface),
        );
    }

    main_row.height(Length::Fill).into()
}

fn view_list<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let header = text("Codex").size(14).color(theme::TEXT_SECONDARY);

    let mut items = column![].spacing(theme::SPACING_XS);

    if project.codex_entries.is_empty() {
        items = items.push(text("No codex entries").size(13).color(theme::TEXT_MUTED));
    } else {
        for entry in &project.codex_entries {
            let is_active = state
                .tabs
                .active_tab()
                .map_or(false, |t| t.id == entry.id);
            let style = if is_active {
                theme::list_item_active
                    as fn(&iced::Theme, button::Status) -> button::Style
            } else {
                theme::list_item
            };
            items = items.push(
                button(text(&entry.label).size(13))
                    .on_press(Message::SelectItem(entry.id.clone()))
                    .width(Length::Fill)
                    .padding([2.0, theme::SPACING_SM])
                    .style(style),
            );
        }
    }

    scrollable(
        column![header, Space::new().height(theme::SPACING_SM), items,]
            .spacing(0.0)
            .padding(theme::SPACING_SM),
    )
    .height(Length::Fill)
    .into()
}

fn view_content<'a>(state: &'a State) -> Element<'a, Message> {
    let bar = tab_bar::view_bar(
        &state.tabs,
        |i| Message::SelectTab(i),
        |i| Message::CloseTab(i),
        |i| Message::TogglePin(i),
    );
    let body = tab_bar::view_content(&state.tabs);

    column![bar, body].height(Length::Fill).into()
}

fn view_interaction<'a>() -> Element<'a, Message> {
    container(
        column![
            text("Interaction")
                .size(14)
                .color(theme::TEXT_SECONDARY),
            Space::new().height(theme::SPACING_MD),
            text("Terminal and chat will appear here.")
                .size(13)
                .color(theme::TEXT_MUTED),
        ]
        .spacing(theme::SPACING_SM)
        .padding(theme::SPACING_LG),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
