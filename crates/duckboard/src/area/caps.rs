//! Caps area — browse the capability tree.

use std::collections::HashSet;

use iced::widget::{column, container, row, scrollable, text, Space};
use iced::{Element, Length};

use crate::data::ProjectData;
use crate::theme;
use crate::widget::{interaction_toggle, tab_bar, tree_view};

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct State {
    pub expanded_nodes: HashSet<String>,
    pub tabs: tab_bar::TabState,
    pub interaction_visible: bool,
    pub interaction_width: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            expanded_nodes: HashSet::new(),
            tabs: tab_bar::TabState::default(),
            interaction_visible: false,
            interaction_width: theme::INTERACTION_COLUMN_WIDTH,
        }
    }
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    ToggleNode(String),
    SelectItem(String),
    SelectTab(usize),
    CloseTab(usize),
    TogglePin(usize),
    InteractionHandle(interaction_toggle::HandleMsg),
    TerminalScroll,
    TabContent(tab_bar::TabContentMsg),
    ToggleEditMode,
    /// A backlink was clicked — path like "tests/auth_test.rs:42".
    BacklinkClicked(String),
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(
    state: &mut State,
    message: Message,
    project: &ProjectData,
    highlighter: &crate::highlight::SyntaxHighlighter,
) {
    match message {
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
        Message::TogglePin(idx) => state.tabs.toggle_pin(idx),
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
        Message::TabContent(tab_bar::TabContentMsg::Structural(
            crate::widget::structural_view::StructMsg::BacklinkClicked(_),
        )) => {
            // Bubbles up — handled in main.rs via BacklinkClicked.
        }
        Message::ToggleEditMode => {
            state.tabs.toggle_edit_mode(highlighter);
        }
        Message::BacklinkClicked(_) => {
            // Handled at the main.rs level.
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

    let toggle = interaction_toggle::view(
        state.interaction_visible,
        state.interaction_width,
        Message::InteractionHandle,
    );

    let mut main_row = row![
        container(list)
            .width(theme::LIST_COLUMN_WIDTH)
            .height(Length::Fill)
            .style(theme::surface),
        divider,
        container(content).width(Length::Fill).height(Length::Fill),
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
    let header = text("Capabilities").size(14).color(theme::TEXT_SECONDARY);

    let tree = if project.cap_tree.is_empty() {
        column![
            text("No capabilities found")
                .size(13)
                .color(theme::TEXT_MUTED)
        ]
        .into()
    } else {
        tree_view::view(
            &project.cap_tree,
            &state.expanded_nodes,
            state.tabs.active_tab().map(|t| t.id.as_str()),
            |id| Message::ToggleNode(id),
            |id| Message::SelectItem(id),
        )
    };

    scrollable(
        column![header, Space::new().height(theme::SPACING_SM), tree,]
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
    let body: Element<'a, tab_bar::TabContentMsg> =
        tab_bar::view_content(&state.tabs);
    let mapped_body: Element<'a, Message> = body.map(|msg| {
        // Intercept backlink clicks so they bubble up properly.
        match &msg {
            tab_bar::TabContentMsg::Structural(
                crate::widget::structural_view::StructMsg::BacklinkClicked(path),
            ) => Message::BacklinkClicked(path.clone()),
            _ => Message::TabContent(msg),
        }
    });

    column![bar, mapped_body].height(Length::Fill).into()
}

fn view_interaction<'a>() -> Element<'a, Message> {
    container(
        column![
            text("Interaction").size(14).color(theme::TEXT_SECONDARY),
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
