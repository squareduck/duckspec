//! duckboard — GUI for the duckspec framework, built with Iced 0.14.


use iced::event;
use iced::keyboard;
use iced::widget::{container, row, stack, Space};
use iced::{Element, Event, Length, Subscription, Task};

mod agent;
mod area;
mod chat_store;
mod data;
pub mod highlight;
mod theme;
mod vcs;
mod watcher;
mod widget;

use area::Area;
use area::interaction::{self, InteractionMode};
use data::ProjectData;
use widget::tab_bar;

// ── Constants for routing keys ──────────────────────────────────────────────

const KEY_CAPS: &str = "caps";
const KEY_CODEX: &str = "codex";

// ── State ────────────────────────────────────────────────────────────────────

struct State {
    active_area: Area,
    project: ProjectData,
    dashboard: area::dashboard::State,
    change: area::change::State,
    caps: area::caps::State,
    codex: area::codex::State,
    file_finder: widget::file_finder::FileFinderState,
    highlighter: highlight::SyntaxHighlighter,
}

impl State {
    fn new() -> Self {
        let project = ProjectData::load();
        tracing::info!(
            root = ?project.duckspec_root,
            caps = project.cap_count,
            codex = project.codex_count,
            changes = project.active_changes.len(),
            "project loaded"
        );
        let mut change = area::change::State::default();
        if let Some(root) = &project.project_root {
            change.changed_files = vcs::changed_files(root);
        }

        // Expand all tree nodes by default.
        let mut caps_expanded = std::collections::HashSet::new();
        data::TreeNode::collect_parent_ids(&project.cap_tree, &mut caps_expanded);
        let caps_state = area::caps::State { expanded_nodes: caps_expanded, ..Default::default() };

        Self {
            active_area: Area::Dashboard,
            project,
            dashboard: area::dashboard::State::default(),
            change,
            caps: caps_state,
            codex: area::codex::State::default(),
            file_finder: widget::file_finder::FileFinderState::default(),
            highlighter: highlight::SyntaxHighlighter::new(),
        }
    }

    /// Resolve a routing key to the corresponding interaction state.
    /// Keys: "caps", "codex", or any other string is a change name.
    fn interaction_mut(&mut self, key: &str) -> Option<&mut interaction::InteractionState> {
        match key {
            KEY_CAPS => Some(&mut self.caps.interaction),
            KEY_CODEX => Some(&mut self.codex.interaction),
            _ => self.change.interactions.get_mut(key),
        }
    }

    /// Get the active area's interaction state and its routing key.
    fn active_interaction(&self) -> Option<(&interaction::InteractionState, &str)> {
        match self.active_area {
            Area::Change => {
                let name = self.change.selected_change.as_deref()?;
                let ix = self.change.interactions.get(name)?;
                Some((ix, name))
            }
            Area::Caps => Some((&self.caps.interaction, KEY_CAPS)),
            Area::Codex => Some((&self.codex.interaction, KEY_CODEX)),
            Area::Dashboard => None,
        }
    }

    /// Get the active area's interaction state mutably and its routing key.
    fn active_interaction_key(&self) -> Option<String> {
        match self.active_area {
            Area::Change => self.change.selected_change.clone(),
            Area::Caps => Some(KEY_CAPS.to_string()),
            Area::Codex => Some(KEY_CODEX.to_string()),
            Area::Dashboard => None,
        }
    }
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Message {
    AreaSelected(Area),
    Refresh,
    Dashboard(area::dashboard::Message),
    Change(area::change::Message),
    Caps(area::caps::Message),
    Codex(area::codex::Message),
    // File finder
    FileFinder(widget::file_finder::Msg),
    // File watcher
    FileChanged(Vec<watcher::FileEvent>),
    // Keyboard
    KeyPress(keyboard::Key, keyboard::Modifiers, Option<String>),
    // Per-instance PTY events (key identifies the interaction)
    PtyEvent(String, widget::terminal::PtyEvent),
    // Per-instance agent events (key identifies the interaction)
    AgentEvent(String, agent::AgentEvent),
    // System theme changed
    ThemeChanged(theme::ColorMode),
}

// ── Update ───────────────────────────────────────────────────────────────────

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::AreaSelected(area) => {
            state.active_area = area;
        }
        Message::Refresh => {
            state.project.reload();
            refresh_open_tabs(state);
            refresh_changed_files(state);
            tracing::info!("project reloaded");
        }
        Message::FileFinder(msg) => {
            use widget::file_finder::Msg;
            match msg {
                Msg::Open => {
                    if let Some(root) = &state.project.project_root {
                        state.file_finder.open(root);
                        // Unfocus terminal in all areas.
                        for ix in state.change.interactions.values_mut() {
                            ix.terminal_focused = false;
                        }
                        state.caps.interaction.terminal_focused = false;
                        state.codex.interaction.terminal_focused = false;
                        return iced::widget::operation::focus("file-finder-input");
                    }
                }
                Msg::Close => {
                    state.file_finder.close();
                }
                Msg::QueryChanged(q) => {
                    state.file_finder.set_query(q);
                }
                Msg::SelectNext => {
                    state.file_finder.select_next();
                }
                Msg::SelectPrev => {
                    state.file_finder.select_prev();
                }
                Msg::Confirm => {
                    if let Some(rel_path) = state.file_finder.selected_path() {
                        if let Some(root) = &state.project.project_root {
                            let abs = root.join(&rel_path);
                            if let Ok(content) = std::fs::read_to_string(&abs) {
                                let id = format!("file:{}", rel_path.display());
                                let title = rel_path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| rel_path.display().to_string());
                                let tabs = match state.active_area {
                                    Area::Change => &mut state.change.tabs,
                                    Area::Caps => &mut state.caps.tabs,
                                    Area::Codex => &mut state.codex.tabs,
                                    Area::Dashboard => {
                                        state.active_area = Area::Change;
                                        &mut state.change.tabs
                                    }
                                };
                                tabs.open_file(id.clone(), title, content);
                                if let Some(tab) = tabs.file_tabs.iter_mut().find(|t| t.id == id)
                                    && let tab_bar::TabView::Editor { editor, .. } = &mut tab.view {
                                        rehighlight(editor, &id, &state.highlighter);
                                    }
                            }
                        }
                        state.file_finder.close();
                    }
                }
            }
        }
        Message::FileChanged(events) => {
            tracing::debug!(count = events.len(), "file watcher events received");
            let duckspec_root = state.project.duckspec_root.as_deref();
            let mut tree_changed = false;

            for event in &events {
                match event {
                    watcher::FileEvent::Modified(path) => {
                        if let Some(root) = duckspec_root {
                            if let Ok(rel) = path.strip_prefix(root) {
                                let id = rel.to_string_lossy().to_string();
                                if let Some(content) = state.project.read_artifact(&id) {
                                    state.change.tabs.refresh_content(&id, content.clone(), &state.highlighter);
                                    state.caps.tabs.refresh_content(&id, content.clone(), &state.highlighter);
                                    state.codex.tabs.refresh_content(&id, content, &state.highlighter);
                                }
                            }
                            if path.starts_with(root) {
                                tree_changed = true;
                            }
                        }
                    }
                    watcher::FileEvent::Removed(path) => {
                        if let Some(root) = duckspec_root {
                            if let Ok(rel) = path.strip_prefix(root) {
                                let id = rel.to_string_lossy().to_string();
                                state.change.tabs.close_by_id(&id);
                                state.caps.tabs.close_by_id(&id);
                                state.codex.tabs.close_by_id(&id);
                            }
                            if path.starts_with(root) {
                                tree_changed = true;
                            }
                        }
                    }
                }
            }

            if tree_changed {
                state.project.reload();
                tracing::debug!("project reloaded (file watcher)");
            }

            refresh_changed_files(state);
        }
        Message::Dashboard(msg) => {
            match &msg {
                area::dashboard::Message::ChangeClicked(name)
                | area::dashboard::Message::ArchivedChangeClicked(name) => {
                    state.change.selected_change = Some(name.clone());
                    state.active_area = Area::Change;
                }
                area::dashboard::Message::FindFile => {
                    return update(
                        state,
                        Message::FileFinder(widget::file_finder::Msg::Open),
                    );
                }
                area::dashboard::Message::GoToCaps => {
                    state.active_area = Area::Caps;
                }
                area::dashboard::Message::GoToCodex => {
                    state.active_area = Area::Codex;
                }
            }
            area::dashboard::update(&mut state.dashboard, msg);
        }
        Message::Change(msg) => {
            area::change::update(&mut state.change, msg, &state.project, &state.highlighter);
        }
        Message::Caps(msg) => {
            area::caps::update(&mut state.caps, msg, &state.project, &state.highlighter);
        }
        Message::Codex(msg) => {
            area::codex::update(&mut state.codex, msg, &state.project, &state.highlighter);
        }
        // Per-instance PTY events
        Message::PtyEvent(key, evt) => {
            use widget::terminal::PtyEvent;
            let Some(ix) = state.interaction_mut(&key) else { return Task::none() };
            match evt {
                PtyEvent::Ready(writer, master) => {
                    if let Some(ref mut ts) = ix.terminal {
                        ts.set_writer(writer.into_writer());
                        ts.set_master(master.into_master());
                        tracing::info!(key, "PTY writer ready");
                    }
                }
                PtyEvent::Output(bytes) => {
                    if let Some(ref mut ts) = ix.terminal {
                        ts.feed(&bytes);
                    }
                }
                PtyEvent::Exited => {
                    tracing::info!(key, "PTY child exited");
                    ix.terminal = None;
                    ix.terminal_focused = false;
                }
            }
        }
        // Per-instance agent events
        Message::AgentEvent(key, evt) => {
            use agent::AgentEvent;
            let Some(ix) = state.interaction_mut(&key) else { return Task::none() };
            match evt {
                AgentEvent::Ready(handle) => {
                    ix.agent_handle = Some(handle);
                    tracing::info!(key, "agent handle ready");
                }
                AgentEvent::CommandsAvailable(commands) => {
                    tracing::info!(key, count = commands.len(), "slash commands discovered");
                    ix.chat_commands = commands;
                }
                AgentEvent::ContentDelta { text } => {
                    if let Some(session) = &mut ix.chat_session {
                        session.pending_text.push_str(&text);
                    }
                }
                AgentEvent::ToolUse { id, name, input } => {
                    if let Some(session) = &mut ix.chat_session {
                        flush_pending_text(session);
                        session.messages.push(chat_store::ChatMessage {
                            role: chat_store::Role::Assistant,
                            content: vec![chat_store::ContentBlock::ToolUse { id, name, input }],
                            timestamp: String::new(),
                        });
                    }
                }
                AgentEvent::ToolResult { id, name, output } => {
                    if let Some(session) = &mut ix.chat_session {
                        session.messages.push(chat_store::ChatMessage {
                            role: chat_store::Role::Assistant,
                            content: vec![chat_store::ContentBlock::ToolResult { id, name, output }],
                            timestamp: String::new(),
                        });
                    }
                }
                AgentEvent::TurnComplete => {
                    if let Some(session) = &mut ix.chat_session {
                        flush_pending_text(session);
                        session.is_streaming = false;
                        if let Err(e) = chat_store::save_session(session) {
                            tracing::error!("failed to save chat session: {e}");
                        }
                    }
                }
                AgentEvent::Error(msg) => {
                    tracing::error!(key, "agent error: {msg}");
                    if let Some(session) = &mut ix.chat_session {
                        session.is_streaming = false;
                        session.messages.push(chat_store::ChatMessage {
                            role: chat_store::Role::System,
                            content: vec![chat_store::ContentBlock::Text(format!("Error: {msg}"))],
                            timestamp: String::new(),
                        });
                    }
                }
                AgentEvent::UsageUpdate { model, input_tokens, output_tokens, context_window } => {
                    if let Some(m) = model {
                        ix.agent_model = m;
                    }
                    if input_tokens > 0 {
                        ix.agent_input_tokens = input_tokens;
                    }
                    if output_tokens > 0 {
                        ix.agent_output_tokens = output_tokens;
                    }
                    if let Some(cw) = context_window {
                        ix.agent_context_window = cw;
                    }
                }
                AgentEvent::ProcessExited => {
                    tracing::info!(key, "agent process exited");
                    ix.agent_handle = None;
                    if let Some(session) = &mut ix.chat_session {
                        session.is_streaming = false;
                    }
                }
            }
            interaction::rebuild_chat_editor(ix);
            if !ix.chat_session.as_ref().is_some_and(|s| s.is_streaming) {
                ix.esc_count = 0;
            }
        }
        Message::ThemeChanged(mode) => {
            theme::set_mode(mode);
            rehighlight_all(state);
        }
        Message::KeyPress(key, mods, text) => {
            // Cmd+P: open file finder.
            if mods.command() && key == keyboard::Key::Character("p".into()) {
                return update(state, Message::FileFinder(widget::file_finder::Msg::Open));
            }

            // When file finder is visible, route navigation keys.
            if state.file_finder.visible {
                use keyboard::key::Named;
                match &key {
                    keyboard::Key::Named(Named::Escape) => {
                        let _ = update(state, Message::FileFinder(widget::file_finder::Msg::Close));
                    }
                    keyboard::Key::Named(Named::Enter) => {
                        let _ = update(state, Message::FileFinder(widget::file_finder::Msg::Confirm));
                    }
                    keyboard::Key::Named(Named::ArrowDown) => {
                        let _ = update(state, Message::FileFinder(widget::file_finder::Msg::SelectNext));
                    }
                    keyboard::Key::Named(Named::ArrowUp) => {
                        let _ = update(state, Message::FileFinder(widget::file_finder::Msg::SelectPrev));
                    }
                    _ if mods.control() && key == keyboard::Key::Character("n".into()) => {
                        let _ = update(state, Message::FileFinder(widget::file_finder::Msg::SelectNext));
                    }
                    _ if mods.control() && key == keyboard::Key::Character("p".into()) => {
                        let _ = update(state, Message::FileFinder(widget::file_finder::Msg::SelectPrev));
                    }
                    _ => {}
                }
                return Task::none();
            }

            // Get the active area's interaction state for keyboard routing.
            let active_info = state.active_interaction().map(|(i, _key)| {
                let agent_chat_active = i.visible
                    && i.mode == InteractionMode::AgentChat
                    && i.chat_session.is_some();
                let terminal_focused = i.terminal_focused;
                let is_streaming = i.chat_session.as_ref().is_some_and(|s| s.is_streaming);
                let completion_visible = i.chat_completion.visible;
                (agent_chat_active, terminal_focused, is_streaming, completion_visible)
            });
            // We need the key separately (can't hold borrow across mutable calls).
            let active_key = state.active_interaction_key();

            if let (Some((agent_chat_active, terminal_focused, is_streaming, completion_visible)), Some(routing_key)) = (active_info, &active_key) {
                // Agent chat completion keyboard shortcuts.
                if agent_chat_active && completion_visible {
                    use keyboard::key::Named;
                    let completion_msg = match &key {
                        keyboard::Key::Named(Named::Tab) => {
                            Some(widget::agent_chat::Msg::CompletionAccept)
                        }
                        keyboard::Key::Named(Named::Escape) => {
                            Some(widget::agent_chat::Msg::CompletionDismiss)
                        }
                        _ if mods.control() && key == keyboard::Key::Character("n".into()) => {
                            Some(widget::agent_chat::Msg::CompletionNext)
                        }
                        _ if mods.control() && key == keyboard::Key::Character("p".into()) => {
                            Some(widget::agent_chat::Msg::CompletionPrev)
                        }
                        _ => None,
                    };
                    if let Some(msg) = completion_msg {
                        return dispatch_interaction_msg(state, routing_key, interaction::Msg::AgentChat(msg));
                    }
                }

                // Agent chat: Esc-Esc to cancel streaming.
                if agent_chat_active
                    && key == keyboard::Key::Named(keyboard::key::Named::Escape)
                    && is_streaming {
                        if let Some(ix) = state.interaction_mut(routing_key) {
                            ix.esc_count += 1;
                            if ix.esc_count >= 2 {
                                return dispatch_interaction_msg(state, routing_key,
                                    interaction::Msg::AgentChat(widget::agent_chat::Msg::CancelPressed));
                            }
                        }
                        return Task::none();
                    }

                // Reset esc counter on any non-Esc key.
                if agent_chat_active
                    && key != keyboard::Key::Named(keyboard::key::Named::Escape)
                    && let Some(ix) = state.interaction_mut(routing_key) {
                        ix.esc_count = 0;
                    }

                // Agent chat: Enter sends, Shift+Enter inserts newline.
                if agent_chat_active
                    && key == keyboard::Key::Named(keyboard::key::Named::Enter)
                {
                    if mods.shift() {
                        if let Some(ix) = state.interaction_mut(routing_key) {
                            ix.chat_input.perform(
                                iced::widget::text_editor::Action::Edit(
                                    iced::widget::text_editor::Edit::Enter,
                                ),
                            );
                        }
                    } else {
                        return dispatch_interaction_msg(state, routing_key,
                            interaction::Msg::AgentChat(widget::agent_chat::Msg::SendPressed));
                    }
                    return Task::none();
                }

                // Terminal keyboard capture.
                if terminal_focused
                    && let Some(ix) = state.interaction_mut(routing_key)
                        && let Some(ref mut ts) = ix.terminal {
                            ts.write_key(key, mods, text.as_deref());
                        }
            }
        }
    }
    Task::none()
}

/// Dispatch an interaction message to the appropriate area by routing key.
fn dispatch_interaction_msg(state: &mut State, key: &str, msg: interaction::Msg) -> Task<Message> {
    match key {
        KEY_CAPS => update(state, Message::Caps(area::caps::Message::Interaction(msg))),
        KEY_CODEX => update(state, Message::Codex(area::codex::Message::Interaction(msg))),
        _ => update(state, Message::Change(area::change::Message::Interaction(msg))),
    }
}

/// Re-highlight all open tabs (e.g. after a theme switch).
fn rehighlight_all(state: &mut State) {
    for tabs in [&mut state.change.tabs, &mut state.caps.tabs, &mut state.codex.tabs] {
        let all_tabs = tabs
            .preview
            .iter_mut()
            .chain(tabs.file_tabs.iter_mut());
        for tab in all_tabs {
            match &mut tab.view {
                tab_bar::TabView::Editor { editor, .. }
                | tab_bar::TabView::Diff { editor, .. } => {
                    rehighlight(editor, &tab.id, &state.highlighter);
                }
            }
        }
    }
}

/// Re-read content for all open text tabs from disk.
fn refresh_open_tabs(state: &mut State) {
    for tabs in [&mut state.change.tabs, &mut state.caps.tabs, &mut state.codex.tabs] {
        let all_tabs = tabs
            .preview
            .iter_mut()
            .chain(tabs.file_tabs.iter_mut());
        for tab in all_tabs {
            if let tab_bar::TabView::Editor { .. } = &tab.view
                && let Some(content) = state.project.read_artifact(&tab.id) {
                    tabs_refresh_single(tab, content, &state.highlighter);
                }
        }
    }
}

fn tabs_refresh_single(
    tab: &mut tab_bar::Tab,
    new_content: String,
    highlighter: &highlight::SyntaxHighlighter,
) {
    if let tab_bar::TabView::Editor { editor, .. } = &mut tab.view {
        *editor = widget::text_edit::EditorState::new(&new_content);
        rehighlight(editor, &tab.id, highlighter);
    }
}

/// Apply an editor action to the active tab's editor state.
pub fn handle_editor_action(
    tabs: &mut tab_bar::TabState,
    action: widget::text_edit::EditorAction,
    highlighter: &highlight::SyntaxHighlighter,
) {
    use widget::text_edit::EditorAction;

    let tab = match tabs.active_tab_mut() {
        Some(t) => t,
        None => return,
    };
    let (editor, tab_id) = match &mut tab.view {
        tab_bar::TabView::Editor { editor, .. } => (editor, tab.id.as_str()),
        tab_bar::TabView::Diff { editor, .. } => (editor, tab.id.as_str()),
    };

    let mutates_text = matches!(
        action,
        EditorAction::Insert(_)
            | EditorAction::Paste(_)
            | EditorAction::Backspace
            | EditorAction::Delete
            | EditorAction::Enter
            | EditorAction::Cut
            | EditorAction::Undo
            | EditorAction::Redo
    );

    match action {
        EditorAction::Insert(ch) => editor.insert_char(ch),
        EditorAction::Paste(text) => editor.insert_text(&text),
        EditorAction::Backspace => editor.backspace(),
        EditorAction::Delete => editor.delete(),
        EditorAction::Enter => editor.insert_char('\n'),
        EditorAction::MoveLeft(sel) => editor.move_left(sel),
        EditorAction::MoveRight(sel) => editor.move_right(sel),
        EditorAction::MoveUp(sel) => editor.move_up(sel),
        EditorAction::MoveDown(sel) => editor.move_down(sel),
        EditorAction::MoveHome(sel) => editor.move_home(sel),
        EditorAction::MoveEnd(sel) => editor.move_end(sel),
        EditorAction::MoveWordLeft(sel) => editor.move_word_left(sel),
        EditorAction::MoveWordRight(sel) => editor.move_word_right(sel),
        EditorAction::SelectAll => editor.select_all(),
        EditorAction::Copy => {}
        EditorAction::Cut => {
            editor.delete_selection();
        }
        EditorAction::Undo => editor.undo(),
        EditorAction::Redo => editor.redo(),
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
        EditorAction::SaveRequested => {
            tracing::info!("save requested (not yet implemented)");
        }
    }

    if mutates_text {
        rehighlight(editor, tab_id, highlighter);
    }
}

/// (Re-)compute syntax highlighting for the given editor state.
pub fn rehighlight(
    editor: &mut widget::text_edit::EditorState,
    tab_id: &str,
    highlighter: &highlight::SyntaxHighlighter,
) {
    let path_str = tab_id.strip_prefix("file:").unwrap_or(tab_id);
    let ext = std::path::Path::new(path_str)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("txt");
    let syntax = highlighter.find_syntax(ext);
    editor.highlight_spans = Some(highlighter.highlight_lines(&editor.lines, syntax));
}

/// Refresh the VCS changed files list.
fn refresh_changed_files(state: &mut State) {
    if let Some(root) = &state.project.project_root {
        state.change.changed_files = vcs::changed_files(root);
    }
}

// ── Artifact tab helper ─────────────────────────────────────────────────────

/// Open a file as a text editor tab. Called from area update functions.
pub fn open_artifact_tab(
    tabs: &mut tab_bar::TabState,
    id: String,
    title: String,
    source: String,
    _artifact_id: &str,
    highlighter: &highlight::SyntaxHighlighter,
) {
    tabs.open_preview(id.clone(), title, source);
    if let Some(tab) = tabs.preview.as_mut()
        && tab.id == id
            && let tab_bar::TabView::Editor { editor, .. } = &mut tab.view {
                rehighlight(editor, &id, highlighter);
            }
}

// ── Agent helpers ───────────────────────────────────────────────────────────

fn flush_pending_text(session: &mut chat_store::ChatSession) {
    if !session.pending_text.is_empty() {
        let text = std::mem::take(&mut session.pending_text);
        session.messages.push(chat_store::ChatMessage {
            role: chat_store::Role::Assistant,
            content: vec![chat_store::ContentBlock::Text(text)],
            timestamp: String::new(),
        });
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

fn view(state: &State) -> Element<'_, Message> {
    let sidebar =
        widget::sidebar::view(&state.active_area, Message::AreaSelected, Message::Refresh);

    let area_content: Element<'_, Message> = match state.active_area {
        Area::Dashboard => {
            area::dashboard::view(&state.dashboard, &state.project, &state.change.changed_files)
                .map(Message::Dashboard)
        }
        Area::Change => {
            area::change::view(&state.change, &state.project).map(Message::Change)
        }
        Area::Caps => {
            area::caps::view(&state.caps, &state.project).map(Message::Caps)
        }
        Area::Codex => {
            area::codex::view(&state.codex, &state.project).map(Message::Codex)
        }
    };

    let sidebar_divider = container(Space::new().height(Length::Fill))
        .width(1.0)
        .style(theme::divider);
    let main_view = row![sidebar, sidebar_divider, area_content];

    if state.file_finder.visible {
        let overlay = widget::file_finder::view(&state.file_finder).map(Message::FileFinder);
        stack![main_view, overlay].into()
    } else {
        main_view.into()
    }
}

// ── Subscription ────────────────────────────────────────────────────────────

fn subscription(state: &State) -> Subscription<Message> {
    let mut subs = vec![];

    // File watcher: active when project root is known.
    if let Some(root) = state.project.project_root.as_ref() {
        subs.push(
            watcher::watch_subscription(root.clone()).map(Message::FileChanged),
        );
    }

    // Per-change PTY subscriptions.
    for (name, ix) in &state.change.interactions {
        if ix.terminal.is_some() {
            let key = format!("pty:change:{name}");
            subs.push(widget::terminal::pty_subscription(key).map(tagged_pty));
        }
    }
    // Caps / Codex PTY subscriptions.
    if state.caps.interaction.terminal.is_some() {
        subs.push(widget::terminal::pty_subscription(format!("pty:{KEY_CAPS}")).map(tagged_pty));
    }
    if state.codex.interaction.terminal.is_some() {
        subs.push(widget::terminal::pty_subscription(format!("pty:{KEY_CODEX}")).map(tagged_pty));
    }

    // Per-change agent subscriptions.
    if let Some(root) = state.project.project_root.as_ref() {
        for (name, ix) in &state.change.interactions {
            if ix.chat_session.is_some() {
                let key = format!("agent:change:{name}");
                subs.push(agent::agent_subscription(key, root.clone()).map(tagged_agent));
            }
        }
        if state.caps.interaction.chat_session.is_some() {
            subs.push(
                agent::agent_subscription(format!("agent:{KEY_CAPS}"), root.clone()).map(tagged_agent),
            );
        }
        if state.codex.interaction.chat_session.is_some() {
            subs.push(
                agent::agent_subscription(format!("agent:{KEY_CODEX}"), root.clone()).map(tagged_agent),
            );
        }
    }

    // Global keyboard events.
    subs.push(event::listen_raw(handle_key_event));

    // Poll system dark/light mode.
    subs.push(theme_subscription());

    Subscription::batch(subs)
}

fn theme_subscription() -> Subscription<Message> {
    Subscription::run(theme_detect_stream).map(Message::ThemeChanged)
}

fn theme_detect_stream() -> impl iced::futures::Stream<Item = theme::ColorMode> {
    use iced::futures::stream::{self, StreamExt};
    use std::sync::atomic::{AtomicU8, Ordering};
    static LAST: AtomicU8 = AtomicU8::new(u8::MAX);
    stream::unfold((), |()| async {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let current = theme::detect_mode();
        Some((current, ()))
    })
    .filter(move |current| {
        let cur_val = *current as u8;
        let prev_val = LAST.swap(cur_val, Ordering::Relaxed);
        async move { prev_val != cur_val }
    })
}

// Non-capturing mapper functions for Subscription::map.
// The key embedded in the tuple carries the routing info.
fn tagged_pty((key, e): (String, widget::terminal::PtyEvent)) -> Message {
    // Strip the "pty:" or "pty:change:" prefix to get the routing key.
    let routing_key = key.strip_prefix("pty:change:").unwrap_or(
        key.strip_prefix("pty:").unwrap_or(&key)
    ).to_string();
    Message::PtyEvent(routing_key, e)
}
fn tagged_agent((key, e): (String, agent::AgentEvent)) -> Message {
    let routing_key = key.strip_prefix("agent:change:").unwrap_or(
        key.strip_prefix("agent:").unwrap_or(&key)
    ).to_string();
    Message::AgentEvent(routing_key, e)
}

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    // Detect system dark/light mode before creating the window.
    theme::set_mode(theme::detect_mode());
    tracing::info!(mode = ?theme::mode(), "duckboard starting");

    iced::application(State::new, update, view)
        .subscription(subscription)
        .title("duckboard")
        .theme(theme_fn)
        .window_size((1200.0, 800.0))
        .run()
}

fn theme_fn(_state: &State) -> iced::Theme {
    theme::app_theme()
}

fn handle_key_event(event: Event, _status: event::Status, _window: iced::window::Id) -> Option<Message> {
    if let Event::Keyboard(keyboard::Event::KeyPressed {
        key, modifiers, text, ..
    }) = event
    {
        Some(Message::KeyPress(key, modifiers, text.map(|s| s.to_string())))
    } else {
        None
    }
}
