//! Caps area — browse the capability tree.

use std::collections::HashSet;

use iced::widget::{column, container, row, scrollable, text, Space};
use iced::{Element, Length};

use crate::data::ProjectData;
use crate::theme;
use crate::widget::{collapsible, interaction_toggle, tab_bar, tree_view};

use super::interaction::{self, InteractionMode, InteractionState};

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
            let is_mode_switch = matches!(msg, interaction::Msg::SwitchMode(_));
            let just_opened = interaction::update(&mut state.interaction, msg);

            if just_opened && state.interaction.mode == InteractionMode::Terminal {
                interaction::spawn_terminal(&mut state.interaction);
            }
            if is_mode_switch && state.interaction.mode == InteractionMode::Terminal {
                interaction::spawn_terminal(&mut state.interaction);
            }

            let wants_agent = (just_opened || is_mode_switch) && state.interaction.mode == InteractionMode::AgentChat;
            if wants_agent && state.interaction.chat_session.is_none() {
                interaction::spawn_agent_session(&mut state.interaction, "caps", project.project_root.as_deref());
            }

            state.interaction.terminal_focused = state.interaction.visible
                && state.interaction.mode == InteractionMode::Terminal;
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
    let list = view_list(state, project);
    let content = view_content(state);
    let divider = container(Space::new().height(Length::Fill))
        .width(1.0)
        .style(theme::divider);

    let toggle = interaction_toggle::view(
        state.interaction.visible,
        state.interaction.width,
        |m| Message::Interaction(interaction::Msg::Handle(m)),
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

    if state.interaction.visible {
        let interaction_col = interaction::view_column(&state.interaction, Message::Interaction);
        main_row = main_row.push(
            container(interaction_col)
                .width(state.interaction.width)
                .height(Length::Fill)
                .style(theme::surface),
        );
    }

    main_row.height(Length::Fill).into()
}

fn view_list<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let tree = if project.cap_tree.is_empty() {
        column![
            text("No capabilities found")
                .size(theme::FONT_MD)
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
