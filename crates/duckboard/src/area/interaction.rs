//! Shared interaction state — terminal + agent chat — used by Change, Caps, and Codex areas.

use iced::widget::text_editor;
use iced::Element;

use crate::agent::{AgentHandle, SlashCommand};
use crate::chat_store::ChatSession;
use crate::highlight::SyntaxHighlighter;
use crate::theme;
use crate::widget::{agent_chat, interaction_toggle, text_edit::{Block, EditorState}};

// ── Interaction mode ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionMode {
    Terminal,
    AgentChat,
}

// ── Session controls (which buttons to show) ────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionControls {
    /// Show a session dropdown + "+" new-session button.
    Multi,
    /// Show a "Clear" button that resets the single session.
    Single,
}

// ── Agent session (per-session bundle) ──────────────────────────────────────

pub struct AgentSession {
    pub session: ChatSession,
    pub agent_handle: Option<AgentHandle>,
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

impl AgentSession {
    /// Create a fresh session for a scope.
    pub fn new(scope: String) -> Self {
        Self::from_session(ChatSession::new(scope))
    }

    /// Wrap a loaded ChatSession with fresh UI state.
    pub fn from_session(session: ChatSession) -> Self {
        Self {
            session,
            agent_handle: None,
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

// ── Interaction state ───────────────────────────────────────────────────────

pub struct InteractionState {
    pub visible: bool,
    pub width: f32,
    pub mode: InteractionMode,
    // Terminal
    pub terminal: Option<crate::widget::terminal::TerminalState>,
    pub terminal_focused: bool,
    // Agent sessions (sorted newest-first).
    pub sessions: Vec<AgentSession>,
    pub active_session: usize,
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            visible: false,
            width: theme::INTERACTION_COLUMN_WIDTH,
            mode: InteractionMode::AgentChat,
            terminal: None,
            terminal_focused: false,
            sessions: Vec::new(),
            active_session: 0,
        }
    }
}

impl InteractionState {
    pub fn active(&self) -> Option<&AgentSession> {
        self.sessions.get(self.active_session)
    }

    pub fn active_mut(&mut self) -> Option<&mut AgentSession> {
        self.sessions.get_mut(self.active_session)
    }

    pub fn find_session_mut(&mut self, id: &str) -> Option<&mut AgentSession> {
        self.sessions.iter_mut().find(|s| s.session.id == id)
    }

    pub fn find_session_index(&self, id: &str) -> Option<usize> {
        self.sessions.iter().position(|s| s.session.id == id)
    }
}

// ── Shared messages ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Msg {
    Handle(interaction_toggle::HandleMsg),
    SwitchMode(InteractionMode),
    AgentChat(agent_chat::Msg),
    TerminalScroll,
    /// Create a new agent session for the current scope. Handled by area.
    NewSession,
    /// Switch the active agent session by id. Handled by area.
    SelectSession(String),
    /// Reset the active session (single-session UIs). Handled by area.
    ClearSession,
}

// ── Update helpers ──────────────────────────────────────────────────────────

/// Handle an interaction message. Returns `true` if the panel was just toggled open.
/// NewSession / SelectSession / ClearSession are ignored here — areas handle them.
pub fn update(state: &mut InteractionState, msg: Msg, highlighter: &SyntaxHighlighter) -> bool {
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
            handle_agent_chat(state, chat_msg, highlighter);
        }
        Msg::TerminalScroll => {
            if let Some(ref mut ts) = state.terminal {
                ts.apply_scroll();
            }
        }
        Msg::NewSession | Msg::SelectSession(_) | Msg::ClearSession => {
            // Area-handled.
        }
    }
    just_opened
}

fn handle_agent_chat(state: &mut InteractionState, msg: agent_chat::Msg, highlighter: &SyntaxHighlighter) {
    let Some(ax) = state.active_mut() else { return };
    match msg {
        agent_chat::Msg::EditorAction(action) => {
            if ax.chat_completion.visible {
                match &action {
                    text_editor::Action::Move(text_editor::Motion::Up) => {
                        completion_prev(ax);
                        return;
                    }
                    text_editor::Action::Move(text_editor::Motion::Down) => {
                        completion_next(ax);
                        return;
                    }
                    _ => {}
                }
            }
            if matches!(action, text_editor::Action::Edit(text_editor::Edit::Enter)) {
                return;
            }
            ax.chat_input.perform(action);
            let input_text = ax.chat_input.text();
            let trimmed = input_text.trim_end();
            if trimmed.starts_with('/') && !trimmed.contains(' ') {
                ax.chat_completion.visible = true;
                ax.chat_completion.selected = 0;
            } else {
                ax.chat_completion.visible = false;
            }
        }
        agent_chat::Msg::CompletionNext => completion_next(ax),
        agent_chat::Msg::CompletionPrev => completion_prev(ax),
        agent_chat::Msg::CompletionAccept => completion_accept(ax),
        agent_chat::Msg::CompletionDismiss => {
            ax.chat_completion.visible = false;
        }
        agent_chat::Msg::ChatAction(idx, action) => {
            for (i, editor) in ax.chat_editors.iter_mut().enumerate() {
                if i != idx {
                    editor.anchor = None;
                }
            }
            if let Some(editor) = ax.chat_editors.get_mut(idx) {
                handle_chat_action_on(editor, action);
            }
        }
        agent_chat::Msg::ToggleCollapse(idx) => {
            if let Some(collapsed) = ax.chat_collapsed.get_mut(idx) {
                *collapsed = !*collapsed;
            }
        }
        agent_chat::Msg::SendPressed => {
            if let Some(handle) = &ax.agent_handle {
                let text = ax.chat_input.text();
                let text = text.trim().to_string();
                if !text.is_empty() {
                    ax.session.messages.push(crate::chat_store::ChatMessage {
                        role: crate::chat_store::Role::User,
                        content: vec![crate::chat_store::ContentBlock::Text(text.clone())],
                        timestamp: String::new(),
                    });
                    ax.session.is_streaming = true;
                    ax.session.pending_text.clear();
                    handle.send_prompt(text, None);
                    ax.chat_input = text_editor::Content::new();
                    ax.chat_completion.visible = false;
                    rebuild_chat_editor(ax, highlighter);
                }
            }
        }
        agent_chat::Msg::CancelPressed => {
            if let Some(handle) = &ax.agent_handle {
                handle.cancel();
            }
        }
    }
}

// ── Chat editor ─────────────────────────────────────────────────────────────

/// Rebuild the per-block chat editors for the given session.
pub fn rebuild_chat_editor(ax: &mut AgentSession, highlighter: &SyntaxHighlighter) {
    let new_blocks = agent_chat::build_chat_blocks(&ax.session);

    let old_len = ax.chat_collapsed.len();
    ax.chat_collapsed.resize(new_blocks.len(), false);
    for (i, block) in new_blocks.iter().enumerate().skip(old_len) {
        ax.chat_collapsed[i] = matches!(
            block.kind,
            crate::widget::text_edit::BlockKind::ToolUse | crate::widget::text_edit::BlockKind::ToolResult
        ) && !block.lines.is_empty();
    }

    let mut new_editors = Vec::with_capacity(new_blocks.len());
    for (i, block) in new_blocks.iter().enumerate() {
        if i < ax.chat_editors.len() && i < ax.chat_blocks.len()
            && ax.chat_blocks[i].lines == block.lines
        {
            let existing = std::mem::replace(
                &mut ax.chat_editors[i],
                EditorState::new(""),
            );
            new_editors.push(existing);
        } else {
            let content = block.lines.join("\n");
            let mut editor = EditorState::new(&content);
            let syntax = highlighter.find_syntax("md");
            editor.highlight_spans = Some(highlighter.highlight_lines(&editor.lines, syntax));
            new_editors.push(editor);
        }
    }

    ax.chat_editors = new_editors;
    ax.chat_blocks = new_blocks;
}

fn handle_chat_action_on(
    editor: &mut EditorState,
    action: crate::widget::text_edit::EditorAction,
) {
    // Chat editors are read-only — skip mutating actions.
    if !action.is_mutating() {
        editor.apply_action(action);
    }
}

// ── Agent chat keyboard routing ────────────────────────────────────────────

/// Result of handling an agent-chat keyboard event.
pub enum AgentChatKeyResult {
    /// The key was consumed; caller should return `Task::none()`.
    Handled,
    /// The key maps to a chat message to dispatch through the update cycle.
    Dispatch(agent_chat::Msg),
    /// The key was not consumed by agent chat keyboard handling.
    NotHandled,
}

/// Handle agent-chat-specific keyboard shortcuts: completion navigation,
/// Esc-Esc cancel, Enter to send, Shift+Enter for newline. Returns how the
/// caller should proceed.
pub fn handle_agent_chat_key(
    ix: &mut InteractionState,
    key: &iced::keyboard::Key,
    mods: iced::keyboard::Modifiers,
) -> AgentChatKeyResult {
    use iced::keyboard;
    use iced::keyboard::key::Named;

    let Some(ax) = ix.active_mut() else {
        return AgentChatKeyResult::NotHandled;
    };

    // Completion shortcuts (Tab, Esc, Ctrl+N/P) when popup is visible.
    if ax.chat_completion.visible {
        let completion_msg = match key {
            keyboard::Key::Named(Named::Tab) => Some(agent_chat::Msg::CompletionAccept),
            keyboard::Key::Named(Named::Escape) => Some(agent_chat::Msg::CompletionDismiss),
            _ if mods.control() && *key == keyboard::Key::Character("n".into()) => {
                Some(agent_chat::Msg::CompletionNext)
            }
            _ if mods.control() && *key == keyboard::Key::Character("p".into()) => {
                Some(agent_chat::Msg::CompletionPrev)
            }
            _ => None,
        };
        if let Some(msg) = completion_msg {
            return AgentChatKeyResult::Dispatch(msg);
        }
    }

    // Esc-Esc to cancel streaming.
    if *key == keyboard::Key::Named(Named::Escape) && ax.session.is_streaming {
        ax.esc_count += 1;
        if ax.esc_count >= 2 {
            return AgentChatKeyResult::Dispatch(agent_chat::Msg::CancelPressed);
        }
        return AgentChatKeyResult::Handled;
    }

    // Reset esc counter on any non-Esc key.
    if *key != keyboard::Key::Named(Named::Escape) {
        ax.esc_count = 0;
    }

    // Enter sends; Shift+Enter inserts newline.
    if *key == keyboard::Key::Named(Named::Enter) {
        if mods.shift() {
            ax.chat_input.perform(
                iced::widget::text_editor::Action::Edit(
                    iced::widget::text_editor::Edit::Enter,
                ),
            );
        } else {
            return AgentChatKeyResult::Dispatch(agent_chat::Msg::SendPressed);
        }
        return AgentChatKeyResult::Handled;
    }

    AgentChatKeyResult::NotHandled
}

// ── Completion helpers ──────────────────────────────────────────────────────

fn completion_next(ax: &mut AgentSession) {
    let input_text = ax.chat_input.text();
    let query = input_text.trim_end().trim_start_matches('/');
    let count = agent_chat::filter_commands(&ax.chat_commands, query).len();
    if count > 0 {
        ax.chat_completion.selected = (ax.chat_completion.selected + 1) % count;
    }
}

fn completion_prev(ax: &mut AgentSession) {
    let input_text = ax.chat_input.text();
    let query = input_text.trim_end().trim_start_matches('/');
    let count = agent_chat::filter_commands(&ax.chat_commands, query).len();
    if count > 0 {
        ax.chat_completion.selected = if ax.chat_completion.selected == 0 {
            count - 1
        } else {
            ax.chat_completion.selected - 1
        };
    }
}

fn completion_accept(ax: &mut AgentSession) {
    let input_text = ax.chat_input.text();
    let query = input_text.trim_end().trim_start_matches('/');
    let filtered = agent_chat::filter_commands(&ax.chat_commands, query);
    let selected = ax.chat_completion.selected.min(filtered.len().saturating_sub(1));
    if let Some(&(cmd_idx, _)) = filtered.get(selected) {
        let cmd_name = &ax.chat_commands[cmd_idx].name;
        let new_text = format!("/{} ", cmd_name);
        ax.chat_input = text_editor::Content::with_text(&new_text);
        ax.chat_input.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
    }
    ax.chat_completion.visible = false;
}

// ── High-level update with side effects ────────────────────────────────────

/// Handle an interaction message with the standard side effects: spawn a
/// terminal or ensure agent sessions when the panel opens or mode switches.
/// Suitable for the common `other =>` arm shared by Caps, Codex, and Change.
pub fn update_with_side_effects(
    state: &mut InteractionState,
    msg: Msg,
    scope: &str,
    project_root: Option<&std::path::Path>,
    highlighter: &SyntaxHighlighter,
) {
    let is_mode_switch = matches!(msg, Msg::SwitchMode(_));
    let just_opened = update(state, msg, highlighter);

    if (just_opened || is_mode_switch) && state.mode == InteractionMode::Terminal {
        spawn_terminal(state);
    }

    if (just_opened || is_mode_switch) && state.mode == InteractionMode::AgentChat {
        ensure_sessions(state, scope, project_root, highlighter);
    }

    state.terminal_focused = state.visible && state.mode == InteractionMode::Terminal;
}

// ── Session management ─────────────────────────────────────────────────────

/// Clear and reset the active session for single-session areas (Caps, Codex).
pub fn clear_single_session(
    ix: &mut InteractionState,
    scope: &str,
    project_root: Option<&std::path::Path>,
) {
    if ix.sessions.is_empty() {
        ix.sessions.push(AgentSession::new(scope.to_string()));
        ix.active_session = 0;
        return;
    }
    let idx = ix.active_session.min(ix.sessions.len() - 1);
    if let Some(ax) = ix.sessions.get(idx) {
        if let Some(handle) = &ax.agent_handle {
            handle.cancel();
        }
        crate::chat_store::delete_session(&ax.session.scope, &ax.session.id, project_root);
    }
    ix.sessions[idx] = AgentSession::new(scope.to_string());
    ix.active_session = idx;
    reconcile_display_names(&mut ix.sessions);
}

// ── Spawn helpers ───────────────────────────────────────────────────────────

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

/// Ensure the interaction has at least one session for the scope.
/// On first call, loads any persisted sessions; if none, creates one empty.
pub fn ensure_sessions(
    state: &mut InteractionState,
    scope: &str,
    project_root: Option<&std::path::Path>,
    highlighter: &SyntaxHighlighter,
) {
    if !state.sessions.is_empty() {
        return;
    }
    let loaded = crate::chat_store::load_sessions_for(scope, project_root);
    if loaded.is_empty() {
        let mut ax = AgentSession::new(scope.to_string());
        reconcile_display_names(std::slice::from_mut(&mut ax));
        state.sessions.push(ax);
    } else {
        for session in loaded {
            let mut ax = AgentSession::from_session(session);
            rebuild_chat_editor(&mut ax, highlighter);
            state.sessions.push(ax);
        }
        // load_sessions_for already reconciled; no-op here.
    }
    state.active_session = 0;
}

/// Re-run display-name reconciliation on a slice of `AgentSession`.
/// Call after inserting a new session or promoting scopes.
pub fn reconcile_display_names(sessions: &mut [AgentSession]) {
    // Temporarily move out the ChatSession fields to satisfy the chat_store
    // signature that takes `&mut [ChatSession]`.
    // Simpler: iterate and build a Vec of mutable refs is not straightforward,
    // so we inline the logic here against AgentSession directly.
    use std::collections::HashMap;
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, ax) in sessions.iter().enumerate() {
        let prefix = crate::chat_store::minute_prefix_public(ax.session.created_at_nanos);
        groups.entry(prefix).or_default().push(i);
    }
    for (_prefix, mut indices) in groups {
        indices.sort_by_key(|&i| sessions[i].session.created_at_nanos);
        if indices.len() == 1 {
            let i = indices[0];
            let minute = crate::chat_store::minute_prefix_public(sessions[i].session.created_at_nanos);
            sessions[i].session.display_name = format!("{} {}", minute, sessions[i].session.scope);
        } else {
            for (n, i) in indices.iter().enumerate() {
                let minute = crate::chat_store::minute_prefix_public(sessions[*i].session.created_at_nanos);
                sessions[*i].session.display_name =
                    format!("{} #{} {}", minute, n + 1, sessions[*i].session.scope);
            }
        }
    }
}

// ── Shared area layout ────────────────────────────────────────────────────

/// Standard area layout: list | divider | content | toggle | [interaction column].
/// Used by Caps and Codex (and potentially others with the same structure).
pub fn area_layout<'a, M: 'a + Clone>(
    list: Element<'a, M>,
    content: Element<'a, M>,
    interaction: &'a InteractionState,
    controls: SessionControls,
    wrap: impl Fn(Msg) -> M + 'a + Clone,
) -> Element<'a, M> {
    use iced::widget::{container, row, Space};
    use iced::Length;

    let divider = container(Space::new().height(Length::Fill))
        .width(1.0)
        .style(crate::theme::divider);

    let toggle = crate::widget::interaction_toggle::view(
        interaction.visible,
        interaction.width,
        {
            let w = wrap.clone();
            move |m| w(Msg::Handle(m))
        },
    );

    let mut main_row = row![
        container(list)
            .width(crate::theme::LIST_COLUMN_WIDTH)
            .height(Length::Fill)
            .style(crate::theme::surface),
        divider,
        container(content).width(Length::Fill).height(Length::Fill),
        toggle,
    ];

    if interaction.visible {
        let interaction_col = view_column(interaction, wrap, controls);
        main_row = main_row.push(
            container(interaction_col)
                .width(interaction.width)
                .height(Length::Fill)
                .style(crate::theme::surface),
        );
    }

    main_row.height(Length::Fill).into()
}

// ── View ────────────────────────────────────────────────────────────────────

/// View the interaction column content (mode tabs + session controls + terminal/agent chat).
pub fn view_column<'a, M: 'a + Clone>(
    state: &'a InteractionState,
    wrap: impl Fn(Msg) -> M + 'a + Clone,
    controls: SessionControls,
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
            if let Some(ax) = state.active() {
                let status = agent_chat::StatusInfo {
                    is_streaming: ax.session.is_streaming,
                    esc_count: ax.esc_count,
                    model: if ax.agent_model.is_empty() {
                        "\u{2014}".to_string()
                    } else {
                        ax.agent_model.clone()
                    },
                    context_tokens: ax.agent_input_tokens + ax.agent_output_tokens,
                    context_max: ax.agent_context_window,
                };
                let w = wrap.clone();
                let chat_view = agent_chat::view(
                    &ax.session,
                    &ax.chat_blocks,
                    &ax.chat_editors,
                    &ax.chat_collapsed,
                    ax.session.is_streaming,
                    &ax.chat_input,
                    &ax.chat_commands,
                    &ax.chat_completion,
                    status,
                )
                .map(move |m| w(Msg::AgentChat(m)));

                let session_bar = view_session_bar(state, controls, wrap.clone());
                column![session_bar, chat_view].height(iced::Length::Fill).into()
            } else {
                view_placeholder(wrap.clone())
            }
        }
    };

    column![mode_tabs, content].height(iced::Length::Fill).into()
}

fn view_session_bar<'a, M: 'a + Clone>(
    state: &'a InteractionState,
    controls: SessionControls,
    wrap: impl Fn(Msg) -> M + 'a + Clone,
) -> Element<'a, M> {
    use iced::widget::{button, column, container, pick_list, row, text, Space};
    use iced::Length;

    let content_row: iced::widget::Row<'a, M> = match controls {
        SessionControls::Single => {
            let w = wrap.clone();
            let clear_btn = button(
                text("Clear").size(theme::font_sm()),
            )
            .on_press(w(Msg::ClearSession))
            .padding([2.0, theme::SPACING_SM])
            .style(theme::session_bar_button);

            row![Space::new().width(Length::Fill), clear_btn]
                .spacing(theme::SPACING_XS)
                .align_y(iced::Center)
        }
        SessionControls::Multi => {
            let options: Vec<SessionChoice> = state
                .sessions
                .iter()
                .map(|s| SessionChoice {
                    id: s.session.id.clone(),
                    label: s.session.display_name.clone(),
                })
                .collect();
            let selected = state.active().map(|ax| SessionChoice {
                id: ax.session.id.clone(),
                label: ax.session.display_name.clone(),
            });

            let w_sel = wrap.clone();
            let picker = pick_list(options, selected, move |choice: SessionChoice| {
                w_sel(Msg::SelectSession(choice.id))
            })
            .placeholder("Session")
            .width(Length::Fill)
            .text_size(theme::font_sm())
            .padding([2.0, theme::SPACING_SM])
            .style(theme::session_picker)
            .menu_style(theme::session_picker_menu);

            let w_new = wrap.clone();
            let new_btn = button(
                text("+").size(theme::font_sm()),
            )
            .on_press(w_new(Msg::NewSession))
            .padding([2.0, theme::SPACING_SM])
            .style(theme::session_bar_button);

            row![picker, new_btn]
                .spacing(theme::SPACING_XS)
                .align_y(iced::Center)
        }
    };

    let bar_border = container(Space::new().width(Length::Fill).height(1.0))
        .width(Length::Fill)
        .style(theme::divider);

    column![
        container(content_row)
            .padding([theme::SPACING_XS, theme::SPACING_SM])
            .width(Length::Fill)
            .style(theme::surface),
        bar_border,
    ]
    .into()
}

/// Choice type for the session pick_list (needs Display + Eq + Clone).
#[derive(Debug, Clone)]
struct SessionChoice {
    id: String,
    label: String,
}

impl std::fmt::Display for SessionChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

impl PartialEq for SessionChoice {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for SessionChoice {}

fn view_mode_tabs<'a, M: 'a + Clone>(
    active: InteractionMode,
    wrap: impl Fn(Msg) -> M + 'a + Clone,
) -> Element<'a, M> {
    use iced::widget::{button, column, container, row, text, Space};
    use iced::Length;

    let modes = [
        ("Agent", InteractionMode::AgentChat),
        ("Terminal", InteractionMode::Terminal),
    ];

    let mut tabs_row = row![].spacing(0.0).height(32.0);

    for (label, mode) in modes {
        let is_active = active == mode;
        let tab_style = if is_active {
            theme::tab_active as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style
        } else {
            theme::tab_inactive
        };

        let w = wrap.clone();
        let tab_btn = button(text(label).size(theme::font_md()))
            .on_press(w(Msg::SwitchMode(mode)))
            .padding([theme::SPACING_SM, theme::SPACING_MD])
            .style(tab_style);

        let underline_style = if is_active { theme::accent_bar } else { theme::surface };
        let underline = container(Space::new().width(Length::Fill).height(2.0))
            .width(Length::Fill)
            .style(underline_style);

        let sep = container(Space::new().width(1.0).height(Length::Fill))
            .style(theme::divider);
        tabs_row = tabs_row.push(sep);
        tabs_row = tabs_row.push(column![tab_btn, underline].width(Length::Shrink));
    }

    let sep = container(Space::new().width(1.0).height(Length::Fill))
        .style(theme::divider);
    tabs_row = tabs_row.push(sep);

    let bar_border = container(Space::new().width(Length::Fill).height(1.0))
        .width(Length::Fill)
        .style(theme::divider);

    column![
        container(tabs_row).width(Length::Fill).style(theme::surface),
        bar_border,
    ]
    .into()
}

fn view_placeholder<'a, M: 'a>(_wrap: impl Fn(Msg) -> M + 'a) -> Element<'a, M> {
    use iced::widget::{column, container, text, Space};

    container(
        column![
            text("Interaction")
                .size(theme::font_md())
                .color(theme::text_secondary()),
            Space::new().height(theme::SPACING_MD),
            text("Terminal and chat will appear here.")
                .size(theme::font_md())
                .color(theme::text_muted()),
        ]
        .spacing(theme::SPACING_SM)
        .padding(theme::SPACING_LG),
    )
    .width(iced::Length::Fill)
    .height(iced::Length::Fill)
    .into()
}
