//! Shared interaction state — terminal + agent chat — used by Change, Caps, and Codex areas.

use iced::widget::text_editor;
use iced::Element;

use crate::agent::{AgentHandle, SlashCommand};
use crate::chat_store::ChatSession;
use crate::theme;
use crate::widget::{agent_chat, interaction_toggle, text_edit::{Block, EditorState}};

// ── Interaction mode ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionMode {
    Terminal,
    AgentChat,
}

// ── Interaction state ───────────────────────────────────────────────────────

pub struct InteractionState {
    pub visible: bool,
    pub width: f32,
    pub mode: InteractionMode,
    // Terminal
    pub terminal: Option<crate::widget::terminal::TerminalState>,
    pub terminal_focused: bool,
    // Agent
    pub agent_handle: Option<AgentHandle>,
    pub chat_session: Option<ChatSession>,
    pub chat_input: text_editor::Content,
    pub chat_commands: Vec<SlashCommand>,
    pub chat_completion: agent_chat::CompletionState,
    pub chat_blocks: Vec<Block>,
    pub chat_editors: Vec<EditorState>,
    pub chat_collapsed: Vec<bool>,
    pub esc_count: u8,
    pub agent_model: String,
    pub agent_input_tokens: usize,
    pub agent_output_tokens: usize,
    pub agent_context_window: usize,
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            visible: false,
            width: theme::INTERACTION_COLUMN_WIDTH,
            mode: InteractionMode::AgentChat,
            terminal: None,
            terminal_focused: false,
            agent_handle: None,
            chat_session: None,
            chat_input: text_editor::Content::new(),
            chat_commands: Vec::new(),
            chat_completion: agent_chat::CompletionState::default(),
            chat_blocks: Vec::new(),
            chat_editors: Vec::new(),
            chat_collapsed: Vec::new(),
            esc_count: 0,
            agent_model: String::new(),
            agent_input_tokens: 0,
            agent_output_tokens: 0,
            agent_context_window: 200_000,
        }
    }
}

// ── Shared messages ─────────────────────────────────────────────────────────

/// Messages that the interaction column can produce, to be wrapped by each area.
#[derive(Debug, Clone)]
pub enum Msg {
    Handle(interaction_toggle::HandleMsg),
    SwitchMode(InteractionMode),
    AgentChat(agent_chat::Msg),
    TerminalScroll,
}

// ── Update helpers ──────────────────────────────────────────────────────────

/// Handle an interaction message. Returns `true` if the panel was just toggled open.
pub fn update(state: &mut InteractionState, msg: Msg) -> bool {
    let mut just_opened = false;
    match msg {
        Msg::Handle(hmsg) => match hmsg {
            interaction_toggle::HandleMsg::Toggle => {
                state.visible = !state.visible;
                just_opened = state.visible;
            }
            interaction_toggle::HandleMsg::SetWidth(w) => {
                state.width = w;
            }
        },
        Msg::SwitchMode(mode) => {
            state.mode = mode;
        }
        Msg::AgentChat(chat_msg) => {
            handle_agent_chat(state, chat_msg);
        }
        Msg::TerminalScroll => {
            if let Some(ref mut ts) = state.terminal {
                ts.apply_scroll();
            }
        }
    }
    just_opened
}

fn handle_agent_chat(state: &mut InteractionState, msg: agent_chat::Msg) {
    match msg {
        agent_chat::Msg::EditorAction(action) => {
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
            if matches!(action, text_editor::Action::Edit(text_editor::Edit::Enter)) {
                return;
            }
            state.chat_input.perform(action);
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
        agent_chat::Msg::ChatAction(idx, action) => {
            // Clear selection on all other editors so only one is active.
            for (i, editor) in state.chat_editors.iter_mut().enumerate() {
                if i != idx {
                    editor.anchor = None;
                }
            }
            if let Some(editor) = state.chat_editors.get_mut(idx) {
                handle_chat_action_on(editor, action);
            }
        }
        agent_chat::Msg::ToggleCollapse(idx) => {
            if let Some(collapsed) = state.chat_collapsed.get_mut(idx) {
                *collapsed = !*collapsed;
            }
        }
        agent_chat::Msg::SendPressed => {
            // Send via agent handle.
            if let Some(handle) = &state.agent_handle {
                let text = state.chat_input.text();
                let text = text.trim().to_string();
                if !text.is_empty() {
                    if let Some(session) = &mut state.chat_session {
                        session.messages.push(crate::chat_store::ChatMessage {
                            role: crate::chat_store::Role::User,
                            content: vec![crate::chat_store::ContentBlock::Text(text.clone())],
                            timestamp: String::new(),
                        });
                        session.is_streaming = true;
                        session.pending_text.clear();
                    }
                    handle.send_prompt(text, None);
                    state.chat_input = text_editor::Content::new();
                    state.chat_completion.visible = false;
                    rebuild_chat_editor(state);
                }
            }
        }
        agent_chat::Msg::CancelPressed => {
            if let Some(handle) = &state.agent_handle {
                handle.cancel();
            }
        }
    }
}

// ── Chat editor ─────────────────────────────────────────────────────────────

/// Rebuild the per-block chat editors from the current session.
///
/// Preserves existing editor state (cursor, anchor, selection) for blocks
/// whose content hasn't changed, so that in-progress selections aren't
/// wiped during streaming rebuilds.
pub fn rebuild_chat_editor(state: &mut InteractionState) {
    let session = match &state.chat_session {
        Some(s) => s,
        None => {
            state.chat_blocks.clear();
            state.chat_editors.clear();
            state.chat_collapsed.clear();
            return;
        }
    };

    let new_blocks = agent_chat::build_chat_blocks(session);

    // Preserve collapsed state for existing blocks, default new ones.
    let old_len = state.chat_collapsed.len();
    state.chat_collapsed.resize(new_blocks.len(), false);
    for (i, block) in new_blocks.iter().enumerate().skip(old_len) {
        state.chat_collapsed[i] = matches!(
            block.kind,
            crate::widget::text_edit::BlockKind::ToolUse | crate::widget::text_edit::BlockKind::ToolResult
        ) && !block.lines.is_empty();
    }

    // Update editors: reuse existing ones when content is unchanged,
    // only create new EditorState for new or changed blocks.
    let mut new_editors = Vec::with_capacity(new_blocks.len());
    for (i, block) in new_blocks.iter().enumerate() {
        if i < state.chat_editors.len() && i < state.chat_blocks.len()
            && state.chat_blocks[i].lines == block.lines
        {
            // Content unchanged — move the existing editor to preserve state.
            let existing = std::mem::replace(
                &mut state.chat_editors[i],
                EditorState::new(""),
            );
            new_editors.push(existing);
        } else {
            let content = block.lines.join("\n");
            new_editors.push(EditorState::new(&content));
        }
    }

    state.chat_editors = new_editors;
    state.chat_blocks = new_blocks;
}

fn handle_chat_action_on(
    editor: &mut EditorState,
    action: crate::widget::text_edit::EditorAction,
) {
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
        EditorAction::Scroll { dy, dx, viewport_height, content_height, viewport_width, content_width } => {
            let max_y = (content_height - viewport_height).max(0.0);
            editor.scroll_y = (editor.scroll_y + dy).clamp(0.0, max_y);
            let max_x = (content_width - viewport_width).max(0.0);
            editor.scroll_x = (editor.scroll_x + dx).clamp(0.0, max_x);
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
        EditorAction::Copy => {}
        _ => {}
    }
}

// ── Completion helpers ──────────────────────────────────────────────────────

fn completion_next(state: &mut InteractionState) {
    let input_text = state.chat_input.text();
    let query = input_text.trim_end().trim_start_matches('/');
    let count = agent_chat::filter_commands(&state.chat_commands, query).len();
    if count > 0 {
        state.chat_completion.selected = (state.chat_completion.selected + 1) % count;
    }
}

fn completion_prev(state: &mut InteractionState) {
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

fn completion_accept(state: &mut InteractionState) {
    let input_text = state.chat_input.text();
    let query = input_text.trim_end().trim_start_matches('/');
    let filtered = agent_chat::filter_commands(&state.chat_commands, query);
    let selected = state.chat_completion.selected.min(filtered.len().saturating_sub(1));
    if let Some(&(cmd_idx, _)) = filtered.get(selected) {
        let cmd_name = &state.chat_commands[cmd_idx].name;
        let new_text = format!("/{} ", cmd_name);
        state.chat_input = text_editor::Content::with_text(&new_text);
        state.chat_input.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
    }
    state.chat_completion.visible = false;
}

// ── Spawn helpers ───────────────────────────────────────────────────────────

/// Spawn a terminal if one doesn't exist yet.
pub fn spawn_terminal(state: &mut InteractionState) {
    if state.terminal.is_none() {
        match crate::widget::terminal::TerminalState::new() {
            Ok(ts) => {
                state.terminal = Some(ts);
                state.terminal_focused = true;
                tracing::info!("terminal spawned");
            }
            Err(e) => tracing::error!("failed to create terminal: {e}"),
        }
    }
}

/// Create an agent chat session if one doesn't exist.
pub fn spawn_agent_session(state: &mut InteractionState, session_name: &str) {
    if state.chat_session.is_none() {
        let session = crate::chat_store::load_session(session_name)
            .unwrap_or_else(|| crate::chat_store::ChatSession::new(session_name.to_string()));
        state.chat_session = Some(session);
        rebuild_chat_editor(state);
        tracing::info!("agent chat session created for {session_name}");
    }
}

// ── View ────────────────────────────────────────────────────────────────────

/// View the interaction column content (mode tabs + terminal/agent chat).
pub fn view_column<'a, M: 'a + Clone>(
    state: &'a InteractionState,
    wrap: impl Fn(Msg) -> M + 'a + Clone,
) -> Element<'a, M> {
    use iced::widget::column;

    let mode_tabs = view_mode_tabs(state.mode, wrap.clone());

    let content: Element<'a, M> = match state.mode {
        InteractionMode::Terminal => {
            if let Some(ts) = &state.terminal {
                let w = wrap.clone();
                crate::widget::terminal::view_terminal(ts)
                    .map(move |_: ()| w(Msg::TerminalScroll))
            } else {
                view_placeholder(wrap.clone())
            }
        }
        InteractionMode::AgentChat => {
            if let Some(session) = &state.chat_session {
                let status = agent_chat::StatusInfo {
                    is_streaming: session.is_streaming,
                    esc_count: state.esc_count,
                    model: if state.agent_model.is_empty() {
                        "\u{2014}".to_string()
                    } else {
                        state.agent_model.clone()
                    },
                    context_tokens: state.agent_input_tokens + state.agent_output_tokens,
                    context_max: state.agent_context_window,
                };
                let w = wrap.clone();
                agent_chat::view(
                    session,
                    &state.chat_blocks,
                    &state.chat_editors,
                    &state.chat_collapsed,
                    session.is_streaming,
                    &state.chat_input,
                    &state.chat_commands,
                    &state.chat_completion,
                    status,
                )
                .map(move |m| w(Msg::AgentChat(m)))
            } else {
                view_placeholder(wrap.clone())
            }
        }
    };

    column![mode_tabs, content].height(iced::Length::Fill).into()
}

fn view_mode_tabs<'a, M: 'a + Clone>(
    active: InteractionMode,
    wrap: impl Fn(Msg) -> M + 'a + Clone,
) -> Element<'a, M> {
    use iced::widget::{button, container, row, text};

    let terminal_style = if active == InteractionMode::Terminal {
        theme::list_item_active as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style
    } else {
        theme::list_item
    };
    let agent_style = if active == InteractionMode::AgentChat {
        theme::list_item_active as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style
    } else {
        theme::list_item
    };

    let w1 = wrap.clone();
    let w2 = wrap.clone();
    container(
        row![
            button(text("Agent").size(theme::FONT_SM))
                .on_press(w1(Msg::SwitchMode(InteractionMode::AgentChat)))
                .padding([2.0, theme::SPACING_SM])
                .style(agent_style),
            button(text("Terminal").size(theme::FONT_SM))
                .on_press(w2(Msg::SwitchMode(InteractionMode::Terminal)))
                .padding([2.0, theme::SPACING_SM])
                .style(terminal_style),
        ]
        .spacing(theme::SPACING_XS),
    )
    .padding([theme::SPACING_XS, theme::SPACING_SM])
    .style(theme::surface)
    .into()
}

fn view_placeholder<'a, M: 'a>(_wrap: impl Fn(Msg) -> M + 'a) -> Element<'a, M> {
    use iced::widget::{column, container, text, Space};

    container(
        column![
            text("Interaction")
                .size(theme::FONT_MD)
                .color(theme::text_secondary()),
            Space::new().height(theme::SPACING_MD),
            text("Terminal and chat will appear here.")
                .size(theme::FONT_MD)
                .color(theme::text_muted()),
        ]
        .spacing(theme::SPACING_SM)
        .padding(theme::SPACING_LG),
    )
    .width(iced::Length::Fill)
    .height(iced::Length::Fill)
    .into()
}
