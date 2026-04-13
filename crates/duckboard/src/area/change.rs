//! Change area — single change workspace with three-column layout.

use std::collections::HashSet;
use std::path::PathBuf;

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Element, Length};

use crate::chat_store::ChatSession;
use crate::data::{ChangeData, ProjectData};
use crate::theme;
use crate::vcs::{self, ChangedFile, FileStatus};
use crate::widget::{agent_chat, collapsible, interaction_toggle, tab_bar, tree_view};

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionMode {
    Terminal,
    AgentChat,
}

#[derive(Debug, Clone)]
pub struct State {
    pub selected_change: Option<String>,
    pub expanded_sections: HashSet<String>,
    pub expanded_nodes: HashSet<String>,
    pub tabs: tab_bar::TabState,
    pub interaction_visible: bool,
    pub interaction_width: f32,
    pub interaction_mode: InteractionMode,
    pub changed_files: Vec<ChangedFile>,
    pub chat_session: Option<ChatSession>,
    pub chat_input: String,
}

impl Default for State {
    fn default() -> Self {
        let mut sections = HashSet::new();
        sections.insert("overview".to_string());
        sections.insert("capabilities".to_string());
        sections.insert("steps".to_string());
        sections.insert("changed_files".to_string());
        Self {
            selected_change: None,
            expanded_sections: sections,
            expanded_nodes: HashSet::new(),
            tabs: tab_bar::TabState::default(),
            interaction_visible: false,
            interaction_width: theme::INTERACTION_COLUMN_WIDTH,
            interaction_mode: InteractionMode::Terminal,
            changed_files: vec![],
            chat_session: None,
            chat_input: String::new(),
        }
    }
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    SelectChange(String),
    ToggleSection(String),
    ToggleNode(String),
    SelectItem(String),
    SelectTab(usize),
    CloseTab(usize),
    InteractionHandle(interaction_toggle::HandleMsg),
    SwitchInteractionMode(InteractionMode),
    AgentChat(agent_chat::Msg),
    TerminalScroll,
    SelectChangedFile(PathBuf),
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
        Message::SelectChange(name) => {
            state.selected_change = Some(name.clone());
            // Expand all tree nodes for the newly selected change.
            state.expanded_nodes.clear();
            if let Some(change) = project
                .active_changes
                .iter()
                .chain(project.archived_changes.iter())
                .find(|c| c.name == name)
            {
                crate::data::TreeNode::collect_parent_ids(
                    &change.cap_tree,
                    &mut state.expanded_nodes,
                );
            }
        }
        Message::ToggleSection(id) => {
            if !state.expanded_sections.remove(&id) {
                state.expanded_sections.insert(id);
            }
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
        Message::InteractionHandle(msg) => match msg {
            interaction_toggle::HandleMsg::Toggle => {
                state.interaction_visible = !state.interaction_visible;
            }
            interaction_toggle::HandleMsg::SetWidth(w) => {
                state.interaction_width = w;
            }
        },
        Message::SwitchInteractionMode(mode) => {
            state.interaction_mode = mode;
        }
        Message::AgentChat(msg) => match msg {
            agent_chat::Msg::InputChanged(val) => {
                state.chat_input = val;
            }
            agent_chat::Msg::SendPressed | agent_chat::Msg::CancelPressed => {
                // Handled in main.rs where we have access to AcpHandle.
            }
        },
        Message::TerminalScroll => {}
        Message::SelectChangedFile(path) => {
            if let Some(root) = &project.project_root {
                if let Some(diff) = vcs::file_diff(root, &path) {
                    let id = format!("vcs:{}", path.display());
                    let title = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string());
                    state.tabs.open_diff(id, title, diff);
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
        // Mode tabs at top of interaction column.
        let mode_tabs = view_mode_tabs(state.interaction_mode);

        // Content based on selected mode.
        let interaction_content: Element<'a, Message> = match state.interaction_mode {
            InteractionMode::Terminal => {
                if let Some(ts) = terminal {
                    crate::widget::terminal::view_terminal(ts).map(|_: ()| Message::TerminalScroll)
                } else {
                    view_interaction()
                }
            }
            InteractionMode::AgentChat => {
                if let Some(session) = &state.chat_session {
                    agent_chat::view(session, &state.chat_input).map(Message::AgentChat)
                } else {
                    view_interaction()
                }
            }
        };

        let interaction_col = column![mode_tabs, interaction_content].height(Length::Fill);

        main_row = main_row.push(
            container(interaction_col)
                .width(state.interaction_width)
                .height(Length::Fill)
                .style(theme::surface),
        );
    }

    main_row.height(Length::Fill).into()
}

fn view_list<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let mut selector = column![].spacing(theme::SPACING_XS);
    let all_changes: Vec<_> = project
        .active_changes
        .iter()
        .chain(project.archived_changes.iter())
        .collect();

    for ch in &all_changes {
        let is_selected = state
            .selected_change
            .as_ref()
            .map_or(false, |s| s == &ch.name);
        let style = if is_selected {
            theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
        } else {
            theme::list_item
        };
        selector = selector.push(
            button(text(&ch.name).size(theme::FONT_MD))
                .on_press(Message::SelectChange(ch.name.clone()))
                .width(Length::Fill)
                .padding([theme::SPACING_XS, theme::SPACING_SM])
                .style(style),
        );
    }

    let change_picker = collapsible::view(
        "Change",
        state.expanded_sections.contains("picker"),
        Message::ToggleSection("picker".to_string()),
        selector.into(),
    );

    let change = find_change(state, project);
    let mut list_col = column![change_picker].spacing(0.0);

    if let Some(change) = change {
        list_col = list_col.push(view_overview_section(state, change));
        list_col = list_col.push(view_caps_section(state, change));
        list_col = list_col.push(view_steps_section(state, change));
    } else {
        list_col = list_col.push(
            container(
                text("Select a change")
                    .size(theme::FONT_MD)
                    .color(theme::TEXT_MUTED),
            )
            .padding(theme::SPACING_LG),
        );
    }

    // Changed files section (always visible, independent of selected change).
    list_col = list_col.push(view_changed_files_section(state));

    scrollable(list_col)
        .height(Length::Fill)
        .into()
}

fn view_overview_section<'a>(state: &'a State, change: &'a ChangeData) -> Element<'a, Message> {
    let mut items = column![].spacing(theme::SPACING_XS);

    if change.has_proposal {
        let id = format!("{}/proposal.md", change.prefix);
        items = items.push(file_item("proposal.md", &id, state));
    }
    if change.has_design {
        let id = format!("{}/design.md", change.prefix);
        items = items.push(file_item("design.md", &id, state));
    }
    if !change.has_proposal && !change.has_design {
        items = items.push(text("No overview files").size(theme::FONT_MD).color(theme::TEXT_MUTED));
    }

    collapsible::view(
        "Overview",
        state.expanded_sections.contains("overview"),
        Message::ToggleSection("overview".to_string()),
        items.into(),
    )
}

fn view_caps_section<'a>(state: &'a State, change: &'a ChangeData) -> Element<'a, Message> {
    let content = if change.cap_tree.is_empty() {
        column![text("No capability changes").size(theme::FONT_MD).color(theme::TEXT_MUTED)].into()
    } else {
        tree_view::view(
            &change.cap_tree,
            &state.expanded_nodes,
            state.tabs.active_tab().map(|t| t.id.as_str()),
            |id| Message::ToggleNode(id),
            |id| Message::SelectItem(id),
        )
    };

    collapsible::view(
        "Capabilities",
        state.expanded_sections.contains("capabilities"),
        Message::ToggleSection("capabilities".to_string()),
        content,
    )
}

fn view_steps_section<'a>(state: &'a State, change: &'a ChangeData) -> Element<'a, Message> {
    let mut items = column![].spacing(theme::SPACING_XS);

    if change.steps.is_empty() {
        items = items.push(text("No steps").size(theme::FONT_MD).color(theme::TEXT_MUTED));
    } else {
        for step in &change.steps {
            items = items.push(file_item(
                &format!("{:02}. {}", step.number, step.label),
                &step.id,
                state,
            ));
        }
    }

    collapsible::view(
        "Steps",
        state.expanded_sections.contains("steps"),
        Message::ToggleSection("steps".to_string()),
        items.into(),
    )
}

fn view_changed_files_section<'a>(state: &'a State) -> Element<'a, Message> {
    let mut items = column![].spacing(theme::SPACING_XS);

    if state.changed_files.is_empty() {
        items = items.push(text("No changes").size(theme::FONT_MD).color(theme::TEXT_MUTED));
    } else {
        for cf in &state.changed_files {
            let status_char = match cf.status {
                FileStatus::Modified => "M",
                FileStatus::Added => "A",
                FileStatus::Deleted => "D",
            };
            let color = theme::vcs_status_color(&cf.status);
            let tab_id = format!("vcs:{}", cf.path.display());
            let is_active = state.tabs.active_tab().map_or(false, |t| t.id == tab_id);
            let style = if is_active {
                theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
            } else {
                theme::list_item
            };

            let label = row![
                text(status_char)
                    .size(theme::FONT_MD)
                    .font(iced::Font::MONOSPACE)
                    .color(color),
                text(cf.path.display().to_string()).size(theme::FONT_MD),
            ]
            .spacing(theme::SPACING_SM);

            items = items.push(
                button(label)
                    .on_press(Message::SelectChangedFile(cf.path.clone()))
                    .width(Length::Fill)
                    .padding([2.0, theme::SPACING_SM])
                    .style(style),
            );
        }
    }

    collapsible::view(
        "Changed Files",
        state.expanded_sections.contains("changed_files"),
        Message::ToggleSection("changed_files".to_string()),
        items.into(),
    )
}

fn file_item<'a>(label: &str, id: &str, state: &State) -> Element<'a, Message> {
    let is_active = state.tabs.active_tab().map_or(false, |t| t.id == id);
    let style = if is_active {
        theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
    } else {
        theme::list_item
    };
    button(text(label.to_string()).size(theme::FONT_MD))
        .on_press(Message::SelectItem(id.to_string()))
        .width(Length::Fill)
        .padding([2.0, theme::SPACING_SM])
        .style(style)
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

fn view_mode_tabs<'a>(active: InteractionMode) -> Element<'a, Message> {
    let terminal_style = if active == InteractionMode::Terminal {
        theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
    } else {
        theme::list_item
    };
    let agent_style = if active == InteractionMode::AgentChat {
        theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
    } else {
        theme::list_item
    };

    container(
        row![
            button(text("Terminal").size(theme::FONT_SM))
                .on_press(Message::SwitchInteractionMode(InteractionMode::Terminal))
                .padding([2.0, theme::SPACING_SM])
                .style(terminal_style),
            button(text("Agent").size(theme::FONT_SM))
                .on_press(Message::SwitchInteractionMode(InteractionMode::AgentChat))
                .padding([2.0, theme::SPACING_SM])
                .style(agent_style),
        ]
        .spacing(theme::SPACING_XS),
    )
    .padding([theme::SPACING_XS, theme::SPACING_SM])
    .style(theme::surface)
    .into()
}

fn view_interaction<'a>() -> Element<'a, Message> {
    container(
        column![
            text("Interaction").size(theme::FONT_MD).color(theme::TEXT_SECONDARY),
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

fn find_change<'a>(state: &State, project: &'a ProjectData) -> Option<&'a ChangeData> {
    let name = state.selected_change.as_ref()?;
    project
        .active_changes
        .iter()
        .chain(project.archived_changes.iter())
        .find(|c| &c.name == name)
}

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
