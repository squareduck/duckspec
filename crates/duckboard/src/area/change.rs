//! Change area — single change workspace with three-column layout.

use std::collections::HashSet;
use std::path::PathBuf;

use iced::widget::{button, column, container, row, scrollable, svg, text, text_editor, Space};
use iced::widget::text::Wrapping;
use iced::{Element, Length};

use crate::agent::SlashCommand;
use crate::chat_store::ChatSession;
use crate::data::{ChangeData, ProjectData, StepCompletion};
use crate::theme;
use crate::vcs::{self, ChangedFile, FileStatus};
use crate::widget::{agent_chat, collapsible, interaction_toggle, tab_bar, tree_view};

const ICON_BRANCH: &[u8] = include_bytes!("../../assets/icon_branch.svg");
const ICON_FILE: &[u8] = include_bytes!("../../assets/icon_file.svg");
const ICON_SPEC: &[u8] = include_bytes!("../../assets/icon_spec.svg");
const ICON_DOC: &[u8] = include_bytes!("../../assets/icon_doc.svg");
const ICON_SPEC_DELTA: &[u8] = include_bytes!("../../assets/icon_spec_delta.svg");
const ICON_DOC_DELTA: &[u8] = include_bytes!("../../assets/icon_doc_delta.svg");
const ICON_STEP: &[u8] = include_bytes!("../../assets/icon_step.svg");
const ICON_STEP_DONE: &[u8] = include_bytes!("../../assets/icon_step_done.svg");
const ICON_STEP_PARTIAL: &[u8] = include_bytes!("../../assets/icon_step_partial.svg");

const ICON_SIZE: f32 = 14.0;

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionMode {
    Terminal,
    AgentChat,
}

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
    pub chat_input: text_editor::Content,
    pub chat_commands: Vec<SlashCommand>,
    pub chat_completion: agent_chat::CompletionState,
    pub chat_editor: Option<crate::widget::text_edit::EditorState>,
    /// Esc key counter for double-esc-to-cancel. Reset when streaming stops.
    pub esc_count: u8,
    /// Agent model name (from last session).
    pub agent_model: String,
    /// Cumulative input tokens for the session.
    pub agent_input_tokens: usize,
    /// Cumulative output tokens for the session.
    pub agent_output_tokens: usize,
    /// Context window size from model.
    pub agent_context_window: usize,
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
            interaction_mode: InteractionMode::AgentChat,
            changed_files: vec![],
            chat_session: None,
            chat_input: text_editor::Content::new(),
            chat_commands: Vec::new(),
            chat_completion: agent_chat::CompletionState::default(),
            chat_editor: None,
            esc_count: 0,
            agent_model: String::new(),
            agent_input_tokens: 0,
            agent_output_tokens: 0,
            agent_context_window: 200_000,
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
            agent_chat::Msg::EditorAction(action) => {
                // When completion is visible, intercept arrow navigation.
                if state.chat_completion.visible {
                    match &action {
                        text_editor::Action::Move(text_editor::Motion::Up) => {
                            completion_prev(state);
                            return;
                        }
                        text_editor::Action::Move(text_editor::Motion::Down) => {
                            completion_next(state);
                            return;
                        }
                        _ => {}
                    }
                }

                // Skip Enter — it's handled via KeyPress for send/newline logic.
                if matches!(action, text_editor::Action::Edit(text_editor::Edit::Enter)) {
                    return;
                }

                state.chat_input.perform(action);

                // Update completion visibility based on current text.
                let input_text = state.chat_input.text();
                let trimmed = input_text.trim_end();
                if trimmed.starts_with('/') && !trimmed.contains(' ') {
                    state.chat_completion.visible = true;
                    state.chat_completion.selected = 0;
                } else {
                    state.chat_completion.visible = false;
                }
            }
            agent_chat::Msg::CompletionNext => completion_next(state),
            agent_chat::Msg::CompletionPrev => completion_prev(state),
            agent_chat::Msg::CompletionAccept => completion_accept(state),
            agent_chat::Msg::CompletionDismiss => {
                state.chat_completion.visible = false;
            }
            agent_chat::Msg::ChatAction(action) => {
                handle_chat_action(&mut state.chat_editor, action);
            }
            agent_chat::Msg::SendPressed | agent_chat::Msg::CancelPressed => {
                // Handled in main.rs where we have access to AgentHandle.
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
                if let (Some(session), Some(chat_editor)) =
                    (&state.chat_session, &state.chat_editor)
                {
                    let status = agent_chat::StatusInfo {
                        is_streaming: session.is_streaming,
                        esc_count: state.esc_count,
                        model: if state.agent_model.is_empty() {
                            "—".to_string()
                        } else {
                            state.agent_model.clone()
                        },
                        context_tokens: state.agent_input_tokens + state.agent_output_tokens,
                        context_max: state.agent_context_window,
                    };
                    agent_chat::view(
                        session,
                        chat_editor,
                        &state.chat_input,
                        &state.chat_commands,
                        &state.chat_completion,
                        status,
                    )
                    .map(Message::AgentChat)
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
        let icon = svg(svg::Handle::from_memory(ICON_BRANCH))
            .width(ICON_SIZE)
            .height(ICON_SIZE);
        let label = row![icon, text(&ch.name).size(theme::FONT_MD).wrapping(Wrapping::None)]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Center);
        selector = selector.push(
            button(label)
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
            let is_active = state.tabs.active_tab().map_or(false, |t| t.id == step.id);
            let style = if is_active {
                theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
            } else {
                theme::list_item
            };
            let icon_bytes: &[u8] = match step.completion {
                StepCompletion::Done => ICON_STEP_DONE,
                StepCompletion::Partial(_, _) => ICON_STEP_PARTIAL,
                StepCompletion::NoTasks => ICON_STEP,
            };
            let icon = svg(svg::Handle::from_memory(icon_bytes))
                .width(ICON_SIZE)
                .height(ICON_SIZE);
            let label = row![
                icon,
                text(format!("{:02}-{}", step.number, step.label)).size(theme::FONT_MD).wrapping(Wrapping::None),
            ]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Center);
            items = items.push(
                button(label)
                    .on_press(Message::SelectItem(step.id.clone()))
                    .width(Length::Fill)
                    .padding([2.0, theme::SPACING_SM])
                    .style(style),
            );
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
                text(cf.path.display().to_string()).size(theme::FONT_MD).wrapping(Wrapping::None),
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

fn icon_for_artifact(label: &str) -> &'static [u8] {
    match label {
        l if l.starts_with("spec.delta") => ICON_SPEC_DELTA,
        l if l.starts_with("spec") => ICON_SPEC,
        l if l.starts_with("doc.delta") => ICON_DOC_DELTA,
        l if l.starts_with("doc") => ICON_DOC,
        _ => ICON_FILE,
    }
}

fn file_item<'a>(label: &str, id: &str, state: &State) -> Element<'a, Message> {
    let is_active = state.tabs.active_tab().map_or(false, |t| t.id == id);
    let style = if is_active {
        theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
    } else {
        theme::list_item
    };
    let icon = svg(svg::Handle::from_memory(icon_for_artifact(label)))
        .width(ICON_SIZE)
        .height(ICON_SIZE);
    let content = row![icon, text(label.to_string()).size(theme::FONT_MD).wrapping(Wrapping::None)]
        .spacing(theme::SPACING_XS)
        .align_y(iced::Center);
    button(content)
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
            button(text("Agent").size(theme::FONT_SM))
                .on_press(Message::SwitchInteractionMode(InteractionMode::AgentChat))
                .padding([2.0, theme::SPACING_SM])
                .style(agent_style),
            button(text("Terminal").size(theme::FONT_SM))
                .on_press(Message::SwitchInteractionMode(InteractionMode::Terminal))
                .padding([2.0, theme::SPACING_SM])
                .style(terminal_style),
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

// ── Chat editor helpers ────────────────────────────────────────────────────

/// Rebuild the single chat editor from the current session, preserving fold states.
///
/// Auto-scroll behavior:
/// - First creation: scroll to bottom.
/// - If user was already at the bottom: follow new content to bottom.
/// - If user scrolled up to explore: keep their scroll position.
pub fn rebuild_chat_editor(state: &mut State) {
    let session = match &state.chat_session {
        Some(s) => s,
        None => {
            state.chat_editor = None;
            return;
        }
    };

    let new_blocks = agent_chat::build_chat_blocks(session);

    if let Some(existing) = &state.chat_editor {
        let was_at_bottom = existing.is_at_bottom();
        let old_scroll = existing.scroll_y;

        let mut editor = crate::widget::text_edit::EditorState::from_blocks(new_blocks);
        if was_at_bottom {
            editor.scroll_to_bottom();
        } else {
            editor.scroll_y = old_scroll;
        }
        state.chat_editor = Some(editor);
    } else {
        // First creation — scroll to bottom.
        let mut editor = crate::widget::text_edit::EditorState::from_blocks(new_blocks);
        editor.scroll_to_bottom();
        state.chat_editor = Some(editor);
    }
}

fn handle_chat_action(
    editor: &mut Option<crate::widget::text_edit::EditorState>,
    action: crate::widget::text_edit::EditorAction,
) {
    let editor = match editor.as_mut() {
        Some(e) => e,
        None => return,
    };

    use crate::widget::text_edit::EditorAction;
    match action {
        EditorAction::Click(pos) => {
            editor.cursor = pos;
            editor.anchor = None;
        }
        EditorAction::Drag(pos) => {
            if editor.anchor.is_none() {
                editor.anchor = Some(editor.cursor);
            }
            editor.cursor = pos;
        }
        EditorAction::Scroll { dy, viewport_height, content_height } => {
            let max = (content_height - viewport_height).max(0.0);
            editor.scroll_y = (editor.scroll_y + dy).clamp(0.0, max);
        }
        EditorAction::SelectAll => editor.select_all(),
        EditorAction::MoveLeft(sel) => editor.move_left(sel),
        EditorAction::MoveRight(sel) => editor.move_right(sel),
        EditorAction::MoveUp(sel) => editor.move_up(sel),
        EditorAction::MoveDown(sel) => editor.move_down(sel),
        EditorAction::MoveHome(sel) => editor.move_home(sel),
        EditorAction::MoveEnd(sel) => editor.move_end(sel),
        EditorAction::MoveWordLeft(sel) => editor.move_word_left(sel),
        EditorAction::MoveWordRight(sel) => editor.move_word_right(sel),
        EditorAction::Copy => {} // Handled by widget directly
        _ => {}                  // Ignore all edit actions in read-only chat
    }
}

// ── Completion helpers ──────────────────────────────────────────────────────

fn completion_next(state: &mut State) {
    let input_text = state.chat_input.text();
    let query = input_text.trim_end().trim_start_matches('/');
    let count = agent_chat::filter_commands(&state.chat_commands, query).len();
    if count > 0 {
        state.chat_completion.selected = (state.chat_completion.selected + 1) % count;
    }
}

fn completion_prev(state: &mut State) {
    let input_text = state.chat_input.text();
    let query = input_text.trim_end().trim_start_matches('/');
    let count = agent_chat::filter_commands(&state.chat_commands, query).len();
    if count > 0 {
        state.chat_completion.selected = if state.chat_completion.selected == 0 {
            count - 1
        } else {
            state.chat_completion.selected - 1
        };
    }
}

fn completion_accept(state: &mut State) {
    let input_text = state.chat_input.text();
    let query = input_text.trim_end().trim_start_matches('/');
    let filtered = agent_chat::filter_commands(&state.chat_commands, query);
    let selected = state.chat_completion.selected.min(filtered.len().saturating_sub(1));
    if let Some(&(cmd_idx, _)) = filtered.get(selected) {
        let cmd_name = &state.chat_commands[cmd_idx].name;
        let new_text = format!("/{} ", cmd_name);
        state.chat_input = text_editor::Content::with_text(&new_text);
        state
            .chat_input
            .perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
    }
    state.chat_completion.visible = false;
}
