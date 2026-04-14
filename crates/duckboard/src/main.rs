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
use data::ProjectData;
use widget::tab_bar;

// ── State ────────────────────────────────────────────────────────────────────

struct State {
    active_area: Area,
    project: ProjectData,
    dashboard: area::dashboard::State,
    change: area::change::State,
    caps: area::caps::State,
    codex: area::codex::State,
    terminal: Option<widget::terminal::TerminalState>,
    terminal_focused: bool,
    file_finder: widget::file_finder::FileFinderState,
    highlighter: highlight::SyntaxHighlighter,
    agent_handle: Option<agent::AgentHandle>,
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
        let mut caps_state = area::caps::State::default();
        caps_state.expanded_nodes = caps_expanded;

        Self {
            active_area: Area::Dashboard,
            project,
            dashboard: area::dashboard::State::default(),
            change,
            caps: caps_state,
            codex: area::codex::State::default(),
            terminal: None,
            terminal_focused: false,
            file_finder: widget::file_finder::FileFinderState::default(),
            highlighter: highlight::SyntaxHighlighter::new(),
            agent_handle: None,
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
    // Terminal
    TerminalSpawn,
    TerminalScroll,
    PtyEvent(widget::terminal::PtyEvent),
    // Agent chat
    AgentEvent(agent::AgentEvent),
    AgentSpawn,
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
                        state.terminal_focused = false;
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
                                // Open in the active area's tabs.
                                // Non-artifact files opened via file finder go
                                // straight to editor; we don't try to classify
                                // arbitrary project files as duckspec artifacts.
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
                                // Highlight the newly opened file.
                                if let Some(tab) = tabs.file_tabs.iter_mut().find(|t| t.id == id) {
                                    if let tab_bar::TabView::Editor { editor, .. } = &mut tab.view {
                                        rehighlight(editor, &id, &state.highlighter);
                                    }
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
                        // Refresh open tabs whose file was modified.
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
                        // Close tabs for removed files.
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

            // Always refresh VCS status on any file change.
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
            if matches!(msg, area::change::Message::TerminalScroll) {
                let _ = update(state, Message::TerminalScroll);
                return Task::none();
            }
            // Handle agent chat actions that need AgentHandle.
            if let area::change::Message::AgentChat(ref chat_msg) = msg {
                match chat_msg {
                    widget::agent_chat::Msg::SendPressed => {
                        if let Some(handle) = &state.agent_handle {
                            let text = state.change.chat_input.text();
                            let text = text.trim().to_string();
                            if !text.is_empty() {
                                // Add user message to session.
                                if let Some(session) = &mut state.change.chat_session {
                                    session.messages.push(chat_store::ChatMessage {
                                        role: chat_store::Role::User,
                                        content: vec![chat_store::ContentBlock::Text(text.clone())],
                                        timestamp: String::new(),
                                    });
                                    session.is_streaming = true;
                                    session.pending_text.clear();
                                }
                                let context = gather_agent_context(state);
                                handle.send_prompt(text, context);
                                state.change.chat_input =
                                    iced::widget::text_editor::Content::new();
                                state.change.chat_completion.visible = false;
                                area::change::rebuild_chat_editor(
                                    &mut state.change,
                                );
                            }
                        }
                        return Task::none();
                    }
                    widget::agent_chat::Msg::CancelPressed => {
                        if let Some(handle) = &state.agent_handle {
                            handle.cancel();
                        }
                        return Task::none();
                    }
                    _ => {}
                }
            }
            // Handle interaction mode switch — spawn agent if needed.
            let is_agent_chat_msg = matches!(msg, area::change::Message::AgentChat(_));
            let is_mode_switch = matches!(
                msg,
                area::change::Message::SwitchInteractionMode(area::change::InteractionMode::AgentChat)
            );
            let is_toggle = matches!(
                msg,
                area::change::Message::InteractionHandle(
                    widget::interaction_toggle::HandleMsg::Toggle
                )
            );
            area::change::update(&mut state.change, msg, &state.project, &state.highlighter);
            if is_toggle && state.change.interaction_visible && state.terminal.is_none() {
                let _ = update(state, Message::TerminalSpawn);
            }
            // Auto-spawn agent session when switching to agent chat mode or
            // opening the panel (default mode is now AgentChat).
            let wants_agent = is_mode_switch
                || (is_toggle
                    && state.change.interaction_visible
                    && state.change.interaction_mode
                        == area::change::InteractionMode::AgentChat);
            if wants_agent && state.change.chat_session.is_none() {
                let _ = update(state, Message::AgentSpawn);
            }
            state.terminal_focused = state.change.interaction_visible
                && state.change.interaction_mode == area::change::InteractionMode::Terminal;
            // Keep chat input focused during agent chat interactions.
            if state.change.interaction_visible
                && state.change.interaction_mode == area::change::InteractionMode::AgentChat
                && (is_toggle || is_mode_switch || is_agent_chat_msg)
            {
                return iced::widget::operation::focus(widget::agent_chat::INPUT_ID);
            }
        }
        Message::Caps(msg) => {
            if matches!(msg, area::caps::Message::TerminalScroll) {
                let _ = update(state, Message::TerminalScroll);
                return Task::none();
            }
            let is_toggle = matches!(
                msg,
                area::caps::Message::InteractionHandle(
                    widget::interaction_toggle::HandleMsg::Toggle
                )
            );
            area::caps::update(&mut state.caps, msg, &state.project, &state.highlighter);
            if is_toggle && state.caps.interaction_visible && state.terminal.is_none() {
                let _ = update(state, Message::TerminalSpawn);
            }
            state.terminal_focused = state.caps.interaction_visible;
        }
        Message::Codex(msg) => {
            if matches!(msg, area::codex::Message::TerminalScroll) {
                let _ = update(state, Message::TerminalScroll);
                return Task::none();
            }
            let is_toggle = matches!(
                msg,
                area::codex::Message::InteractionHandle(
                    widget::interaction_toggle::HandleMsg::Toggle
                )
            );
            area::codex::update(&mut state.codex, msg, &state.project, &state.highlighter);
            if is_toggle && state.codex.interaction_visible && state.terminal.is_none() {
                let _ = update(state, Message::TerminalSpawn);
            }
            state.terminal_focused = state.codex.interaction_visible;
        }
        // Terminal
        Message::TerminalSpawn => {
            if state.terminal.is_none() {
                match widget::terminal::TerminalState::new() {
                    Ok(ts) => {
                        state.terminal = Some(ts);
                        state.terminal_focused = true;
                        tracing::info!("terminal spawned");
                    }
                    Err(e) => tracing::error!("failed to create terminal: {e}"),
                }
            }
        }
        Message::PtyEvent(evt) => {
            use widget::terminal::PtyEvent;
            match evt {
                PtyEvent::Ready(writer, master) => {
                    if let Some(ref mut ts) = state.terminal {
                        ts.set_writer(writer.into_writer());
                        ts.set_master(master.into_master());
                        tracing::info!("PTY writer ready");
                    }
                }
                PtyEvent::Output(bytes) => {
                    if let Some(ref mut ts) = state.terminal {
                        ts.feed(&bytes);
                    }
                }
                PtyEvent::Exited => {
                    tracing::info!("PTY child exited");
                    state.terminal = None;
                    state.terminal_focused = false;
                }
            }
        }
        Message::TerminalScroll => {
            if let Some(ref mut ts) = state.terminal {
                ts.apply_scroll();
            }
        }
        // Agent chat
        Message::AgentSpawn => {
            if state.change.chat_session.is_none() {
                let change_name = state
                    .change
                    .selected_change
                    .clone()
                    .unwrap_or_else(|| "default".to_string());

                // Try to load persisted session, or create new.
                let session = chat_store::load_session(&change_name)
                    .unwrap_or_else(|| chat_store::ChatSession::new(change_name));
                state.change.chat_session = Some(session);
                area::change::rebuild_chat_editor(&mut state.change);
                tracing::info!("agent chat session created");
            }
        }
        Message::AgentEvent(evt) => {
            use agent::AgentEvent;
            match evt {
                AgentEvent::Ready(handle) => {
                    state.agent_handle = Some(handle);
                    tracing::info!("agent handle ready");
                }
                AgentEvent::CommandsAvailable(commands) => {
                    tracing::info!(count = commands.len(), "slash commands discovered");
                    state.change.chat_commands = commands;
                }
                AgentEvent::ContentDelta { text } => {
                    if let Some(session) = &mut state.change.chat_session {
                        session.pending_text.push_str(&text);
                    }
                }
                AgentEvent::ToolUse { id, name, input } => {
                    if let Some(session) = &mut state.change.chat_session {
                        // Flush any pending text first.
                        flush_pending_text(session);
                        session.messages.push(chat_store::ChatMessage {
                            role: chat_store::Role::Assistant,
                            content: vec![chat_store::ContentBlock::ToolUse { id, name, input }],
                            timestamp: String::new(),
                        });
                    }
                }
                AgentEvent::ToolResult { id, name, output } => {
                    if let Some(session) = &mut state.change.chat_session {
                        session.messages.push(chat_store::ChatMessage {
                            role: chat_store::Role::Assistant,
                            content: vec![chat_store::ContentBlock::ToolResult { id, name, output }],
                            timestamp: String::new(),
                        });
                    }
                }
                AgentEvent::TurnComplete => {
                    if let Some(session) = &mut state.change.chat_session {
                        flush_pending_text(session);
                        session.is_streaming = false;
                        // Persist to disk.
                        if let Err(e) = chat_store::save_session(session) {
                            tracing::error!("failed to save chat session: {e}");
                        }
                    }
                }
                AgentEvent::Error(msg) => {
                    tracing::error!("agent error: {msg}");
                    if let Some(session) = &mut state.change.chat_session {
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
                        state.change.agent_model = m;
                    }
                    if input_tokens > 0 {
                        state.change.agent_input_tokens = input_tokens;
                    }
                    if output_tokens > 0 {
                        state.change.agent_output_tokens = output_tokens;
                    }
                    if let Some(cw) = context_window {
                        state.change.agent_context_window = cw;
                    }
                }
                AgentEvent::ProcessExited => {
                    tracing::info!("agent process exited");
                    state.agent_handle = None;
                    if let Some(session) = &mut state.change.chat_session {
                        session.is_streaming = false;
                    }
                }
            }
            // Sync editor states for any new messages.
            area::change::rebuild_chat_editor(&mut state.change);
            // Reset esc counter when streaming stops.
            if !state.change.chat_session.as_ref().map_or(false, |s| s.is_streaming) {
                state.change.esc_count = 0;
            }
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

            // Agent chat completion keyboard shortcuts.
            let agent_chat_active = state.change.interaction_visible
                && state.change.interaction_mode == area::change::InteractionMode::AgentChat
                && state.change.chat_session.is_some();

            if agent_chat_active && state.change.chat_completion.visible {
                use keyboard::key::Named;
                let completion_msg = match &key {
                    keyboard::Key::Named(Named::Tab) => {
                        Some(widget::agent_chat::Msg::CompletionAccept)
                    }
                    keyboard::Key::Named(Named::Escape) => {
                        Some(widget::agent_chat::Msg::CompletionDismiss)
                    }
                    _ if mods.control()
                        && key == keyboard::Key::Character("n".into()) =>
                    {
                        Some(widget::agent_chat::Msg::CompletionNext)
                    }
                    _ if mods.control()
                        && key == keyboard::Key::Character("p".into()) =>
                    {
                        Some(widget::agent_chat::Msg::CompletionPrev)
                    }
                    _ => None,
                };
                if let Some(msg) = completion_msg {
                    return update(
                        state,
                        Message::Change(area::change::Message::AgentChat(msg)),
                    );
                }
            }

            // Agent chat: Esc-Esc to cancel streaming.
            if agent_chat_active
                && key == keyboard::Key::Named(keyboard::key::Named::Escape)
            {
                let is_streaming = state.change.chat_session
                    .as_ref()
                    .map_or(false, |s| s.is_streaming);
                if is_streaming {
                    state.change.esc_count += 1;
                    if state.change.esc_count >= 2 {
                        // Don't reset esc_count — keep it at 2 so the status
                        // bar shows "cancelling…" until streaming actually stops.
                        return update(
                            state,
                            Message::Change(area::change::Message::AgentChat(
                                widget::agent_chat::Msg::CancelPressed,
                            )),
                        );
                    }
                    return Task::none();
                }
            }

            // Reset esc counter on any non-Esc key.
            if agent_chat_active
                && key != keyboard::Key::Named(keyboard::key::Named::Escape)
            {
                state.change.esc_count = 0;
            }

            // Agent chat: Enter sends, Shift+Enter inserts newline.
            if agent_chat_active
                && key == keyboard::Key::Named(keyboard::key::Named::Enter)
            {
                if mods.shift() {
                    state.change.chat_input.perform(
                        iced::widget::text_editor::Action::Edit(
                            iced::widget::text_editor::Edit::Enter,
                        ),
                    );
                } else {
                    return update(
                        state,
                        Message::Change(area::change::Message::AgentChat(
                            widget::agent_chat::Msg::SendPressed,
                        )),
                    );
                }
                return Task::none();
            }

            // Terminal keyboard capture.
            if state.terminal_focused {
                if let Some(ref mut ts) = state.terminal {
                    ts.write_key(key, mods, text.as_deref());
                }
            }
        }
    }
    Task::none()
}

/// Re-read content for all open text tabs from disk.
fn refresh_open_tabs(state: &mut State) {
    for tabs in [&mut state.change.tabs, &mut state.caps.tabs, &mut state.codex.tabs] {
        let all_tabs = tabs
            .preview
            .iter_mut()
            .chain(tabs.file_tabs.iter_mut());
        for tab in all_tabs {
            if let tab_bar::TabView::Editor { .. } = &tab.view {
                if let Some(content) = state.project.read_artifact(&tab.id) {
                    tabs_refresh_single(tab, content, &state.highlighter);
                }
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
        _ => return,
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
        EditorAction::Copy => {
            // Copy is handled in the widget's on_event (clipboard access).
        }
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
        EditorAction::Scroll { dy, viewport_height, content_height } => {
            let max = (content_height - viewport_height).max(0.0);
            editor.scroll_y = (editor.scroll_y + dy).clamp(0.0, max);
        }
        EditorAction::SaveRequested => {
            // TODO: write editor content back to disk.
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
    if let Some(tab) = tabs.preview.as_mut() {
        if tab.id == id {
            if let tab_bar::TabView::Editor { editor, .. } = &mut tab.view {
                rehighlight(editor, &id, highlighter);
            }
        }
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

fn gather_agent_context(state: &State) -> Option<agent::AgentContext> {
    let project_root = state.project.project_root.as_ref()?.clone();
    let duckspec_root = state.project.duckspec_root.as_ref()?;
    let change_name = state.change.selected_change.as_ref()?;
    let change_dir = duckspec_root.join("changes").join(change_name);

    let changed_files = state
        .change
        .changed_files
        .iter()
        .map(|f| f.path.clone())
        .collect();

    // Read spec content if available.
    let spec_content = std::fs::read_to_string(change_dir.join("proposal.md")).ok();
    let step_content = None; // Could read the latest step file.
    let git_diff = None; // Could compute from vcs module.

    Some(agent::AgentContext {
        project_root,
        change_dir,
        changed_files,
        spec_content,
        step_content,
        git_diff,
    })
}

// ── View ─────────────────────────────────────────────────────────────────────

fn view(state: &State) -> Element<'_, Message> {
    let sidebar =
        widget::sidebar::view(&state.active_area, Message::AreaSelected, Message::Refresh);

    let term = state.terminal.as_ref();
    let area_content: Element<'_, Message> = match state.active_area {
        Area::Dashboard => {
            area::dashboard::view(&state.dashboard, &state.project, &state.change.changed_files)
                .map(Message::Dashboard)
        }
        Area::Change => {
            area::change::view(&state.change, &state.project, term).map(Message::Change)
        }
        Area::Caps => {
            area::caps::view(&state.caps, &state.project, term).map(Message::Caps)
        }
        Area::Codex => {
            area::codex::view(&state.codex, &state.project, term).map(Message::Codex)
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

// ── Main ─────────────────────────────────────────────────────────────────────

// ── Subscription ────────────────────────────────────────────────────────────

fn subscription(state: &State) -> Subscription<Message> {
    let mut subs = vec![];

    // File watcher: active when project root is known.
    if let Some(root) = state.project.project_root.as_ref() {
        subs.push(
            watcher::watch_subscription(root.clone()).map(Message::FileChanged),
        );
    }

    // PTY I/O subscription: active when a terminal exists.
    if state.terminal.is_some() {
        subs.push(
            widget::terminal::pty_subscription().map(Message::PtyEvent),
        );
    }

    // Agent subscription: active when a chat session exists.
    if state.change.chat_session.is_some() {
        if let Some(root) = state.project.project_root.as_ref() {
            subs.push(
                agent::agent_subscription(root.clone()).map(Message::AgentEvent),
            );
        }
    }

    // Global keyboard events — routing happens in update based on state.
    subs.push(event::listen_raw(handle_key_event));

    Subscription::batch(subs)
}

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();
    tracing::info!("duckboard starting");

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
