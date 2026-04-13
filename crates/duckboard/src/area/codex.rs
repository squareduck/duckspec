//! Codex area — browse codex entries.

use iced::widget::{button, column, container, row, scrollable, svg, text, Space};
use iced::widget::text::Wrapping;
use iced::{Element, Length};

use crate::data::ProjectData;
use crate::theme;
use crate::widget::{collapsible, interaction_toggle, tab_bar};

const ICON_FILE: &[u8] = include_bytes!("../../assets/icon_file.svg");
const ICON_SIZE: f32 = 14.0;

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct State {
    pub section_expanded: bool,
    pub tabs: tab_bar::TabState,
    pub interaction_visible: bool,
    pub interaction_width: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            section_expanded: true,
            tabs: tab_bar::TabState::default(),
            interaction_visible: false,
            interaction_width: theme::INTERACTION_COLUMN_WIDTH,
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
    InteractionHandle(interaction_toggle::HandleMsg),
    TerminalScroll,
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
        Message::InteractionHandle(msg) => match msg {
            interaction_toggle::HandleMsg::Toggle => {
                state.interaction_visible = !state.interaction_visible;
            }
            interaction_toggle::HandleMsg::SetWidth(w) => {
                state.interaction_width = w;
            }
        },
        Message::TerminalScroll => {}
        Message::TabContent(tab_bar::TabContentMsg::EditorAction(action)) => {
            crate::handle_editor_action(&mut state.tabs, action, highlighter);
        }
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(
    state: &'a State,
    project: &'a ProjectData,
    terminal: Option<&'a crate::widget::terminal::TerminalState>,
) -> Element<'a, Message> {
    let list = view_list(state, project);
    let content = view_content(state);
    let divider = container(Space::new().height(Length::Fill))
        .width(1.0)
        .style(theme::divider);

    let toggle =
        interaction_toggle::view(state.interaction_visible, state.interaction_width, Message::InteractionHandle);

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
        let interaction: Element<'a, Message> = if let Some(ts) = terminal {
            crate::widget::terminal::view_terminal(ts).map(|_: ()| Message::TerminalScroll)
        } else {
            view_interaction()
        };
        main_row = main_row.push(
            container(interaction)
                .width(state.interaction_width)
                .height(Length::Fill)
                .style(theme::surface),
        );
    }

    main_row.height(Length::Fill).into()
}

fn view_list<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let mut items = column![].spacing(theme::SPACING_XS);

    if project.codex_entries.is_empty() {
        items = items.push(text("No codex entries").size(theme::FONT_MD).color(theme::TEXT_MUTED));
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
            let icon = svg(svg::Handle::from_memory(ICON_FILE))
                .width(ICON_SIZE)
                .height(ICON_SIZE);
            let label = row![icon, text(&entry.label).size(theme::FONT_MD).wrapping(Wrapping::None)]
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
        .height(Length::Fill)
        .into()
}

fn view_content<'a>(state: &'a State) -> Element<'a, Message> {
    let bar = tab_bar::view_bar(
        &state.tabs,
        |i| Message::SelectTab(i),
        |i| Message::CloseTab(i),
    );
    let body = tab_bar::view_content(&state.tabs).map(Message::TabContent);

    column![bar, body].height(Length::Fill).into()
}

fn view_interaction<'a>() -> Element<'a, Message> {
    container(
        column![
            text("Interaction")
                .size(theme::FONT_MD)
                .color(theme::TEXT_SECONDARY),
            Space::new().height(theme::SPACING_MD),
            text("Terminal and chat will appear here.")
                .size(theme::FONT_MD)
                .color(theme::TEXT_MUTED),
        ]
        .spacing(theme::SPACING_SM)
        .padding(theme::SPACING_LG),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
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
