//! duckboard — GUI for the duckspec framework, built with Iced 0.14.

use iced::event;
use iced::keyboard;
use iced::widget::{Space, column, container, row, stack};
use iced::{Element, Event, Length, Subscription, Task};

mod agent;
mod area;
mod chat_store;
pub mod config;
mod data;
pub mod highlight;
mod theme;
mod vcs;
mod watcher;
mod widget;

use area::Area;
use area::interaction::{self, ActiveTab};
use data::ProjectData;
use widget::tab_bar;

// ── Constants for routing keys ──────────────────────────────────────────────

const KEY_CAPS: &str = "caps";
const KEY_CODEX: &str = "codex";

// ── State ────────────────────────────────────────────────────────────────────

struct State {
    active_area: Area,
    project: ProjectData,
    config: config::Config,
    dashboard: area::dashboard::State,
    change: area::change::State,
    caps: area::caps::State,
    codex: area::codex::State,
    settings: area::settings::State,
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
        let mut change = area::change::State::new(project.project_root.as_deref());
        if let Some(root) = &project.project_root {
            change.set_changed_files(vcs::changed_files(root));
        }

        // Expand all tree nodes by default.
        let mut caps_expanded = std::collections::HashSet::new();
        data::TreeNode::collect_parent_ids(&project.cap_tree, &mut caps_expanded);
        let caps_state = area::caps::State {
            expanded_nodes: caps_expanded,
            ..Default::default()
        };

        let config = config::load();
        theme::set_fonts(&config);
        Self {
            active_area: Area::Dashboard,
            project,
            config,
            dashboard: area::dashboard::State::default(),
            change,
            caps: caps_state,
            codex: area::codex::State::default(),
            settings: area::settings::State::default(),
            file_finder: widget::file_finder::FileFinderState::default(),
            highlighter: highlight::SyntaxHighlighter::new(),
        }
    }

    /// Resolve a scope (bare change name / "caps" / "codex") to its interaction state.
    fn interaction_mut(&mut self, scope: &str) -> Option<&mut interaction::InteractionState> {
        match scope {
            KEY_CAPS => Some(&mut self.caps.interaction),
            KEY_CODEX => Some(&mut self.codex.interaction),
            _ => self.change.interactions.get_mut(scope),
        }
    }

    /// Resolve a stable `InteractionState::instance_id` to its state.
    /// Used for routing long-lived subscription events (PTY, agent) that must
    /// survive scope renames like exploration→change promotion.
    fn interaction_mut_by_ix_id(
        &mut self,
        ix_id: u64,
    ) -> Option<&mut interaction::InteractionState> {
        if self.caps.interaction.instance_id == ix_id {
            return Some(&mut self.caps.interaction);
        }
        if self.codex.interaction.instance_id == ix_id {
            return Some(&mut self.codex.interaction);
        }
        self.change
            .interactions
            .values_mut()
            .find(|ix| ix.instance_id == ix_id)
    }

    /// Resolve a composite routing key `<instance_id>/<session_id>` to the session bundle.
    fn agent_session_mut(&mut self, key: &str) -> Option<&mut interaction::AgentSession> {
        let (ix_id_str, session_id) = key.split_once('/')?;
        let ix_id: u64 = ix_id_str.parse().ok()?;
        let ix = self.interaction_mut_by_ix_id(ix_id)?;
        ix.find_session_mut(session_id)
    }

    /// Get the active area's interaction state and its scope.
    fn active_interaction(&self) -> Option<(&interaction::InteractionState, &str)> {
        match self.active_area {
            Area::Change => {
                let name = self.change.selected_change.as_deref()?;
                let ix = self.change.interactions.get(name)?;
                Some((ix, name))
            }
            Area::Caps => Some((&self.caps.interaction, KEY_CAPS)),
            Area::Codex => Some((&self.codex.interaction, KEY_CODEX)),
            Area::Dashboard | Area::Settings => None,
        }
    }

    /// Get the active area's scope (for looking up the interaction state).
    fn active_interaction_key(&self) -> Option<String> {
        match self.active_area {
            Area::Change => self.change.selected_change.clone(),
            Area::Caps => Some(KEY_CAPS.to_string()),
            Area::Codex => Some(KEY_CODEX.to_string()),
            Area::Dashboard | Area::Settings => None,
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
    // Per-terminal PTY events. `ix_id` is the stable `InteractionState::instance_id`,
    // `terminal_id` identifies the specific terminal tab within that interaction.
    PtyEvent(u64, u64, widget::terminal::PtyEvent),
    // Clipboard → PTY paste (scope name identifies the interaction).
    TerminalPaste(String, Option<String>),
    // Per-instance agent events. Key format: `<instance_id>/<session_id>`.
    AgentEvent(String, agent::AgentEvent),
    // Settings
    Settings(area::settings::Message),
    // System theme changed
    ThemeChanged(theme::ColorMode),
    // Animation tick for the streaming indicator; only fires while a session
    // is streaming (see `subscription`).
    StreamTick,
}

// ── Update ───────────────────────────────────────────────────────────────────

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::AreaSelected(area) => {
            state.active_area = area;
            if area == Area::Settings {
                area::settings::update(
                    &mut state.settings,
                    &mut state.config,
                    area::settings::Message::LoadFonts,
                );
            }
        }
        Message::Refresh => {
            reload_and_reconcile(state);
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
                                    Area::Dashboard | Area::Settings => {
                                        state.active_area = Area::Change;
                                        &mut state.change.tabs
                                    }
                                };
                                tabs.open_file(id.clone(), title, content, Some(abs.clone()));
                                if let Some(tab) = tabs.file_tabs.iter_mut().find(|t| t.id == id)
                                    && let tab_bar::TabView::Editor { editor, .. } = &mut tab.view
                                {
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
            let duckspec_root = state.project.duckspec_root.clone();
            let project_root = state.project.project_root.clone();
            let mut tree_changed = false;
            let mut vcs_state_changed = false;

            for event in &events {
                match event {
                    watcher::FileEvent::Modified(path) => {
                        if let Some(root) = duckspec_root.as_deref() {
                            if let Ok(rel) = path.strip_prefix(root) {
                                let id = rel.to_string_lossy().to_string();
                                if let Some(content) = state.project.read_artifact(&id) {
                                    state.change.tabs.refresh_content(
                                        &id,
                                        content.clone(),
                                        &state.highlighter,
                                    );
                                    state.caps.tabs.refresh_content(
                                        &id,
                                        content.clone(),
                                        &state.highlighter,
                                    );
                                    state.codex.tabs.refresh_content(
                                        &id,
                                        content,
                                        &state.highlighter,
                                    );
                                }
                            }
                            if path.starts_with(root) {
                                tree_changed = true;
                            }
                        }
                        if let Some(root) = project_root.as_deref() {
                            refresh_file_tabs_for_path(state, root, path);
                            refresh_diff_tabs_for_path(state, root, path);
                        }
                    }
                    watcher::FileEvent::Removed(path) => {
                        if let Some(root) = duckspec_root.as_deref() {
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
                        if let Some(root) = project_root.as_deref()
                            && let Ok(rel) = path.strip_prefix(root)
                        {
                            let diff_id = format!("vcs:{}", rel.display());
                            state.change.tabs.close_by_id(&diff_id);
                            state.caps.tabs.close_by_id(&diff_id);
                            state.codex.tabs.close_by_id(&diff_id);
                        }
                    }
                    watcher::FileEvent::VcsStateChanged(path) => {
                        tracing::debug!(path = %path.display(), "git state changed — refreshing");
                        vcs_state_changed = true;
                    }
                }
            }

            if tree_changed && reload_and_reconcile(state) {
                // Tab IDs were rewritten to new archive paths; re-read
                // their content from disk so editors reflect the moved files.
                refresh_open_tabs(state);
            }

            if vcs_state_changed && let Some(root) = project_root.as_deref() {
                refresh_all_diff_tabs(state, root);
            }

            refresh_changed_files(state);
        }
        Message::Dashboard(msg) => {
            match &msg {
                area::dashboard::Message::ChangeClicked(name)
                | area::dashboard::Message::ArchivedChangeClicked(name)
                | area::dashboard::Message::ExplorationClicked(name) => {
                    state.active_area = Area::Change;
                    area::change::update(
                        &mut state.change,
                        area::change::Message::SelectChange(name.clone()),
                        &state.project,
                        &state.highlighter,
                    );
                }
                area::dashboard::Message::AddExploration => {
                    // Delegate to the change area's exploration logic, then switch.
                    area::change::update(
                        &mut state.change,
                        area::change::Message::AddExploration,
                        &state.project,
                        &state.highlighter,
                    );
                    state.active_area = Area::Change;
                }
                area::dashboard::Message::RefreshAudit => {
                    state.project.revalidate();
                }
                area::dashboard::Message::SelectAuditError {
                    change,
                    artifact_id,
                } => {
                    state.active_area = Area::Change;
                    area::change::update(
                        &mut state.change,
                        area::change::Message::OpenArtifact {
                            change: change.clone(),
                            artifact_id: artifact_id.clone(),
                        },
                        &state.project,
                        &state.highlighter,
                    );
                }
            }
            area::dashboard::update(&mut state.dashboard, msg);
        }
        Message::Change(msg) => {
            let needs_focus = is_chat_focus_msg(extract_change_interaction_msg(&msg));
            area::change::update(&mut state.change, msg, &state.project, &state.highlighter);
            if needs_focus {
                return focus_chat_input();
            }
        }
        Message::Caps(msg) => {
            let needs_focus = is_chat_focus_msg(extract_caps_interaction_msg(&msg));
            area::caps::update(&mut state.caps, msg, &state.project, &state.highlighter);
            if needs_focus {
                return focus_chat_input();
            }
        }
        Message::Codex(msg) => {
            let needs_focus = is_chat_focus_msg(extract_codex_interaction_msg(&msg));
            area::codex::update(&mut state.codex, msg, &state.project, &state.highlighter);
            if needs_focus {
                return focus_chat_input();
            }
        }
        Message::Settings(msg) => {
            area::settings::update(&mut state.settings, &mut state.config, msg);
            theme::set_fonts(&state.config);
        }
        // Clipboard → PTY paste.
        Message::TerminalPaste(key, Some(text)) => {
            if let Some(ix) = state.interaction_mut(&key)
                && let Some(tt) = ix.active_terminal_mut()
            {
                tt.state.paste_text(&text);
            }
        }
        Message::TerminalPaste(_, None) => {}
        // Per-terminal PTY events
        Message::PtyEvent(ix_id, terminal_id, evt) => {
            use widget::terminal::PtyEvent;
            let Some(ix) = state.interaction_mut_by_ix_id(ix_id) else {
                return Task::none();
            };
            let Some(idx) = ix.find_terminal_index(terminal_id) else {
                return Task::none();
            };
            match evt {
                PtyEvent::Ready(writer, master) => {
                    if let Some(tt) = ix.terminals.get_mut(idx) {
                        tt.state.set_writer(writer.into_writer());
                        tt.state.set_master(master.into_master());
                        tracing::info!(ix_id, terminal_id, "PTY writer ready");
                    }
                }
                PtyEvent::Output(bytes) => {
                    if let Some(tt) = ix.terminals.get_mut(idx) {
                        tt.state.feed(&bytes);
                    }
                }
                PtyEvent::Exited => {
                    tracing::info!(ix_id, terminal_id, "PTY child exited");
                    ix.terminals.remove(idx);
                    ix.active_tab = interaction::adjust_active_after_remove(ix.active_tab, idx);
                    ix.terminal_focused =
                        ix.visible && matches!(ix.active_tab, ActiveTab::Terminal(_));
                }
            }
        }
        // Per-instance agent events — key is `<scope>/<session_id>`.
        Message::AgentEvent(key, evt) => {
            use agent::AgentEvent;
            let proj_root = state.project.project_root.clone();
            {
                let Some(ax) = state.agent_session_mut(&key) else {
                    return Task::none();
                };
                match evt {
                    AgentEvent::Ready(handle) => {
                        // Seed the worker with a previously-persisted Claude session
                        // id so the next prompt resumes that conversation.
                        if let Some(sid) = ax.session.claude_session_id.clone() {
                            handle.set_session_id(sid);
                        }
                        ax.agent_handle = Some(handle);
                        tracing::info!(key, "agent handle ready");
                    }
                    AgentEvent::CommandsAvailable(commands) => {
                        tracing::info!(key, count = commands.len(), "slash commands discovered");
                        ax.chat_commands = commands;
                    }
                    AgentEvent::ContentDelta { text } => {
                        ax.session.pending_text.push_str(&text);
                    }
                    AgentEvent::ToolUse { id, name, input } => {
                        flush_pending_text(&mut ax.session);
                        ax.session.messages.push(chat_store::ChatMessage {
                            role: chat_store::Role::Assistant,
                            content: vec![chat_store::ContentBlock::ToolUse { id, name, input }],
                            timestamp: String::new(),
                        });
                    }
                    AgentEvent::ToolResult { id, name, output } => {
                        ax.session.messages.push(chat_store::ChatMessage {
                            role: chat_store::Role::Assistant,
                            content: vec![chat_store::ContentBlock::ToolResult {
                                id,
                                name,
                                output,
                            }],
                            timestamp: String::new(),
                        });
                    }
                    AgentEvent::TurnComplete => {
                        flush_pending_text(&mut ax.session);
                        ax.session.is_streaming = false;
                        if let Err(e) = chat_store::save_session(&ax.session, proj_root.as_deref())
                        {
                            tracing::error!("failed to save chat session: {e}");
                        }
                    }
                    AgentEvent::Error(msg) => {
                        tracing::error!(key, "agent error: {msg}");
                        ax.session.is_streaming = false;
                        ax.session.messages.push(chat_store::ChatMessage {
                            role: chat_store::Role::System,
                            content: vec![chat_store::ContentBlock::Text(format!("Error: {msg}"))],
                            timestamp: String::new(),
                        });
                    }
                    AgentEvent::SessionIdUpdated { session_id } => {
                        ax.session.claude_session_id = Some(session_id);
                    }
                    AgentEvent::UsageUpdate {
                        model,
                        input_tokens,
                        output_tokens,
                        context_window,
                    } => {
                        if let Some(m) = model {
                            ax.agent_model = m;
                        }
                        if input_tokens > 0 {
                            ax.agent_input_tokens = input_tokens;
                        }
                        if output_tokens > 0 {
                            ax.agent_output_tokens = output_tokens;
                        }
                        if let Some(cw) = context_window {
                            ax.agent_context_window = cw;
                        }
                    }
                    AgentEvent::ProcessExited => {
                        tracing::info!(key, "agent process exited");
                        ax.agent_handle = None;
                        ax.session.is_streaming = false;
                    }
                }
            }
            let State {
                change,
                caps,
                codex,
                highlighter,
                ..
            } = state;
            let ax = resolve_session_mut(change, caps, codex, &key);
            if let Some(ax) = ax {
                let is_streaming = ax.session.is_streaming;
                interaction::rebuild_chat_editor(ax, highlighter);
                if !is_streaming {
                    ax.esc_count = 0;
                    // Auto-flush a queued message once the current turn is
                    // done (natural completion or user-triggered interrupt).
                    // Only flush if the agent is still attached — on
                    // ProcessExited the handle is gone and we'd lose the text.
                    if ax.agent_handle.is_some()
                        && let Some(q) = ax.queue_editor.take()
                    {
                        let text = q.text();
                        if !text.trim().is_empty() {
                            interaction::send_prompt_text(ax, text, highlighter);
                        }
                    }
                }
            }
        }
        Message::ThemeChanged(mode) => {
            theme::set_mode(mode);
            rehighlight_all(state);
        }
        Message::StreamTick => {
            widget::streaming_indicator::bump_tick();
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
                        let _ = update(
                            state,
                            Message::FileFinder(widget::file_finder::Msg::Confirm),
                        );
                    }
                    keyboard::Key::Named(Named::ArrowDown) => {
                        let _ = update(
                            state,
                            Message::FileFinder(widget::file_finder::Msg::SelectNext),
                        );
                    }
                    keyboard::Key::Named(Named::ArrowUp) => {
                        let _ = update(
                            state,
                            Message::FileFinder(widget::file_finder::Msg::SelectPrev),
                        );
                    }
                    _ if mods.control() && key == keyboard::Key::Character("n".into()) => {
                        let _ = update(
                            state,
                            Message::FileFinder(widget::file_finder::Msg::SelectNext),
                        );
                    }
                    _ if mods.control() && key == keyboard::Key::Character("p".into()) => {
                        let _ = update(
                            state,
                            Message::FileFinder(widget::file_finder::Msg::SelectPrev),
                        );
                    }
                    _ => {}
                }
                return Task::none();
            }

            // Get the active area's interaction state for keyboard routing.
            let active_info = state.active_interaction().map(|(i, _key)| {
                let agent_chat_active =
                    i.visible && i.active_tab == ActiveTab::Chat && i.active().is_some();
                let terminal_focused = i.terminal_focused;
                (agent_chat_active, terminal_focused)
            });
            // We need the key separately (can't hold borrow across mutable calls).
            let active_key = state.active_interaction_key();

            if let (Some((agent_chat_active, terminal_focused, ..)), Some(routing_key)) =
                (active_info, &active_key)
            {
                // Agent chat keyboard shortcuts (completion, esc-cancel, enter-send).
                if agent_chat_active {
                    if let Some(ix) = state.interaction_mut(routing_key) {
                        match interaction::handle_agent_chat_key(ix, &key, mods) {
                            interaction::AgentChatKeyResult::Handled => return Task::none(),
                            interaction::AgentChatKeyResult::Dispatch(msg) => {
                                return dispatch_interaction_msg(
                                    state,
                                    routing_key,
                                    interaction::Msg::AgentChat(msg),
                                );
                            }
                            interaction::AgentChatKeyResult::NotHandled => {}
                        }
                    }

                    // Cmd-N (Ctrl-N off-mac): new chat session in the change area.
                    // Only fires while agent chat is the active interaction so it
                    // doesn't shadow the rest of the app.
                    if state.active_area == Area::Change
                        && mods.command()
                        && key == keyboard::Key::Character("n".into())
                    {
                        return dispatch_interaction_msg(
                            state,
                            routing_key,
                            interaction::Msg::NewSession,
                        );
                    }
                }

                // Terminal keyboard capture.
                if terminal_focused {
                    // Clipboard shortcuts: Cmd+C/V on macOS, Ctrl+Shift+C/V elsewhere.
                    let clipboard_combo = if cfg!(target_os = "macos") {
                        mods.logo() && !mods.control() && !mods.alt() && !mods.shift()
                    } else {
                        mods.control() && mods.shift() && !mods.alt() && !mods.logo()
                    };
                    if clipboard_combo && let keyboard::Key::Character(c) = &key {
                        match c.as_str().to_ascii_lowercase().as_str() {
                            "c" => {
                                let selection = state
                                    .interaction_mut(routing_key)
                                    .and_then(|ix| ix.active_terminal())
                                    .and_then(|tt| tt.state.selection_text());
                                if let Some(text) = selection {
                                    return iced::clipboard::write(text);
                                }
                                return Task::none();
                            }
                            "v" => {
                                let route = routing_key.clone();
                                return iced::clipboard::read()
                                    .map(move |opt| Message::TerminalPaste(route.clone(), opt));
                            }
                            _ => {}
                        }
                    }

                    if let Some(ix) = state.interaction_mut(routing_key)
                        && let Some(tt) = ix.active_terminal_mut()
                    {
                        tt.state.write_key(key, mods, text.as_deref());
                    }
                }
            }
        }
    }
    Task::none()
}

/// Resolve a composite routing key `<instance_id>/<session_id>` to its AgentSession
/// by borrowing only the three area substates. Useful when the caller needs
/// to also hold a borrow on other fields (e.g. `highlighter`) of `State`.
fn resolve_session_mut<'a>(
    change: &'a mut area::change::State,
    caps: &'a mut area::caps::State,
    codex: &'a mut area::codex::State,
    key: &str,
) -> Option<&'a mut interaction::AgentSession> {
    let (ix_id_str, session_id) = key.split_once('/')?;
    let ix_id: u64 = ix_id_str.parse().ok()?;
    let ix = if caps.interaction.instance_id == ix_id {
        &mut caps.interaction
    } else if codex.interaction.instance_id == ix_id {
        &mut codex.interaction
    } else {
        change
            .interactions
            .values_mut()
            .find(|ix| ix.instance_id == ix_id)?
    };
    ix.find_session_mut(session_id)
}

/// Dispatch an interaction message to the appropriate area by routing key.
fn dispatch_interaction_msg(state: &mut State, key: &str, msg: interaction::Msg) -> Task<Message> {
    match key {
        KEY_CAPS => update(state, Message::Caps(area::caps::Message::Interaction(msg))),
        KEY_CODEX => update(
            state,
            Message::Codex(area::codex::Message::Interaction(msg)),
        ),
        _ => update(
            state,
            Message::Change(area::change::Message::Interaction(msg)),
        ),
    }
}

/// Focus the chat input. Used after creating, switching, or clearing a
/// session so the user can immediately type — no extra click required.
fn focus_chat_input() -> Task<Message> {
    iced::widget::operation::focus(widget::agent_chat::CHAT_INPUT_ID)
}

/// True when an interaction message changes the active session in a way that
/// should re-focus the chat input (new session created, current cleared).
fn is_chat_focus_msg(msg: Option<&interaction::Msg>) -> bool {
    matches!(
        msg,
        Some(interaction::Msg::NewSession | interaction::Msg::ClearSession)
    )
}

fn extract_change_interaction_msg(msg: &area::change::Message) -> Option<&interaction::Msg> {
    if let area::change::Message::Interaction(m) = msg {
        Some(m)
    } else {
        None
    }
}

fn extract_caps_interaction_msg(msg: &area::caps::Message) -> Option<&interaction::Msg> {
    if let area::caps::Message::Interaction(m) = msg {
        Some(m)
    } else {
        None
    }
}

fn extract_codex_interaction_msg(msg: &area::codex::Message) -> Option<&interaction::Msg> {
    if let area::codex::Message::Interaction(m) = msg {
        Some(m)
    } else {
        None
    }
}

/// Re-highlight all open tabs and chat editors (e.g. after a theme switch).
///
/// `EditorState::highlight_spans` bake in concrete RGB colors at highlight
/// time, so a theme switch is invisible until every editor is re-highlighted.
fn rehighlight_all(state: &mut State) {
    for tabs in [
        &mut state.change.tabs,
        &mut state.caps.tabs,
        &mut state.codex.tabs,
    ] {
        let all_tabs = tabs.preview.iter_mut().chain(tabs.file_tabs.iter_mut());
        for tab in all_tabs {
            match &mut tab.view {
                tab_bar::TabView::Editor { editor, .. } | tab_bar::TabView::Diff { editor, .. } => {
                    rehighlight(editor, &tab.id, &state.highlighter);
                }
            }
        }
    }

    let md_syntax = state.highlighter.find_syntax("md");
    let rehighlight_session =
        |ax: &mut interaction::AgentSession, highlighter: &highlight::SyntaxHighlighter| {
            ax.chat_input.highlight_spans =
                Some(highlighter.highlight_lines(&ax.chat_input.lines, md_syntax));
            for editor in ax.chat_editors.iter_mut() {
                editor.highlight_spans =
                    Some(highlighter.highlight_lines(&editor.lines, md_syntax));
            }
        };
    for ix in state.change.interactions.values_mut() {
        for ax in ix.sessions.iter_mut() {
            rehighlight_session(ax, &state.highlighter);
        }
    }
    for ax in state.caps.interaction.sessions.iter_mut() {
        rehighlight_session(ax, &state.highlighter);
    }
    for ax in state.codex.interaction.sessions.iter_mut() {
        rehighlight_session(ax, &state.highlighter);
    }
}

/// Reload `ProjectData` and reconcile duckboard-local state: promote a selected
/// exploration if a new change appeared, migrate subscriptions when a change
/// was archived externally, and refresh the obvious-command hint. Returns
/// `true` when tab IDs were rewritten for an external archival, so the caller
/// can refresh open-tab contents from disk.
fn reload_and_reconcile(state: &mut State) -> bool {
    use std::collections::HashSet;

    let old_change_names: HashSet<String> = state
        .project
        .active_changes
        .iter()
        .map(|c| c.name.clone())
        .collect();
    let old_archived_names: HashSet<String> = state
        .project
        .archived_changes
        .iter()
        .map(|c| c.name.clone())
        .collect();

    state.project.reload();

    // Detect new change directories and promote exploration if active.
    if state.change.is_exploration_selected() {
        let new_change = state
            .project
            .active_changes
            .iter()
            .find(|c| !old_change_names.contains(&c.name))
            .map(|c| c.name.clone());

        if let Some(new_name) = new_change
            && let Some(exploration_name) = state.change.selected_change.clone()
        {
            tracing::info!(
                from = exploration_name,
                to = new_name.as_str(),
                "promoting exploration to real change"
            );
            state.change.promote_exploration(
                &exploration_name,
                &new_name,
                state.project.project_root.as_deref(),
            );
        }
    }

    // Detect new archived change directories and migrate subscriptions from
    // the matching active-change name (archival happened externally).
    let new_archived: Vec<String> = state
        .project
        .archived_changes
        .iter()
        .filter(|c| !old_archived_names.contains(&c.name))
        .map(|c| c.name.clone())
        .collect();

    let mut archived_any = false;
    for archived_name in new_archived {
        let Some(base_name) = data::strip_archive_prefix(&archived_name) else {
            continue;
        };
        if state.change.interactions.contains_key(base_name) {
            tracing::info!(
                from = base_name,
                to = archived_name.as_str(),
                "migrating subscriptions to archived change"
            );
            state.change.archive_change(
                base_name,
                &archived_name,
                state.project.project_root.as_deref(),
            );
            archived_any = true;
        }
    }

    area::change::refresh_obvious_command(&mut state.change, &state.project);
    archived_any
}

/// Re-read content for all open text tabs from disk.
fn refresh_open_tabs(state: &mut State) {
    for tabs in [
        &mut state.change.tabs,
        &mut state.caps.tabs,
        &mut state.codex.tabs,
    ] {
        let all_tabs = tabs.preview.iter_mut().chain(tabs.file_tabs.iter_mut());
        for tab in all_tabs {
            if let tab_bar::TabView::Editor { .. } = &tab.view
                && let Some(content) = state.project.read_artifact(&tab.id)
            {
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
    let tab = match tabs.active_tab_mut() {
        Some(t) => t,
        None => return,
    };

    if matches!(action, widget::text_edit::EditorAction::SaveRequested) {
        if let tab_bar::TabView::Editor { editor, path } = &mut tab.view
            && let Some(path) = path.as_ref()
        {
            let text = editor.text();
            match std::fs::write(path, &text) {
                Ok(()) => {
                    editor.dirty = false;
                    tracing::info!(path = %path.display(), "saved file");
                }
                Err(err) => {
                    tracing::error!(path = %path.display(), %err, "failed to save file");
                }
            }
        }
        return;
    }

    if let widget::text_edit::EditorAction::OpenUrl(url) = &action {
        if let Err(err) = opener::open(url) {
            tracing::warn!(%url, %err, "failed to open editor URL");
        }
        return;
    }

    let (editor, tab_id) = match &mut tab.view {
        tab_bar::TabView::Editor { editor, .. } => (editor, tab.id.as_str()),
        tab_bar::TabView::Diff { editor, .. } => (editor, tab.id.as_str()),
    };

    if editor.apply_action(action) {
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
        state.change.set_changed_files(vcs::changed_files(root));
    }
}

/// Re-read any open `file:`-prefixed tabs whose underlying path matches
/// `changed_path`. Used when the watcher reports a file modification.
fn refresh_file_tabs_for_path(
    state: &mut State,
    project_root: &std::path::Path,
    changed_path: &std::path::Path,
) {
    let Ok(rel) = changed_path.strip_prefix(project_root) else {
        return;
    };
    let id = format!("file:{}", rel.display());
    let Ok(content) = std::fs::read_to_string(changed_path) else {
        return;
    };
    for tabs in [
        &mut state.change.tabs,
        &mut state.caps.tabs,
        &mut state.codex.tabs,
    ] {
        tabs.refresh_content(&id, content.clone(), &state.highlighter);
    }
}

/// Rebuild any open `vcs:`-prefixed tabs whose underlying path matches
/// `changed_path`. If the file no longer differs from HEAD, close the tab.
fn refresh_diff_tabs_for_path(
    state: &mut State,
    project_root: &std::path::Path,
    changed_path: &std::path::Path,
) {
    let Ok(rel) = changed_path.strip_prefix(project_root) else {
        return;
    };
    let id = format!("vcs:{}", rel.display());
    rebuild_diff_tab(state, project_root, &id, rel);
}

/// Rebuild every open diff tab — used on VCS state changes (HEAD/index/refs)
/// where the diff baseline shifts for all open diffs at once.
fn refresh_all_diff_tabs(state: &mut State, project_root: &std::path::Path) {
    let ids: Vec<String> = [&state.change.tabs, &state.caps.tabs, &state.codex.tabs]
        .into_iter()
        .flat_map(|tabs| {
            tabs.preview
                .iter()
                .chain(tabs.file_tabs.iter())
                .filter(|t| matches!(t.view, tab_bar::TabView::Diff { .. }))
                .map(|t| t.id.clone())
        })
        .collect();
    let mut seen = std::collections::HashSet::new();
    for id in ids {
        if !seen.insert(id.clone()) {
            continue;
        }
        let Some(rel_str) = id.strip_prefix("vcs:") else {
            continue;
        };
        let rel = std::path::PathBuf::from(rel_str);
        rebuild_diff_tab(state, project_root, &id, &rel);
    }
}

fn rebuild_diff_tab(
    state: &mut State,
    project_root: &std::path::Path,
    id: &str,
    rel: &std::path::Path,
) {
    match widget::diff_view::build_diff_tab(project_root, rel, &state.highlighter) {
        Some(content) => {
            for tabs in [
                &mut state.change.tabs,
                &mut state.caps.tabs,
                &mut state.codex.tabs,
            ] {
                tabs.refresh_diff(
                    id,
                    content.editor.clone(),
                    content.path.clone(),
                    content.status,
                );
            }
        }
        None => {
            for tabs in [
                &mut state.change.tabs,
                &mut state.caps.tabs,
                &mut state.codex.tabs,
            ] {
                tabs.close_by_id(id);
            }
        }
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
    path: Option<std::path::PathBuf>,
    highlighter: &highlight::SyntaxHighlighter,
) {
    tabs.open_preview(id.clone(), title, source, path);
    if let Some(tab) = tabs.preview.as_mut()
        && tab.id == id
        && let tab_bar::TabView::Editor { editor, .. } = &mut tab.view
    {
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
    let next_mode = match theme::mode() {
        theme::ColorMode::Dark => theme::ColorMode::Light,
        theme::ColorMode::Light => theme::ColorMode::Dark,
    };
    let sidebar = widget::sidebar::view(
        &state.active_area,
        Message::AreaSelected,
        Message::Refresh,
        Message::ThemeChanged(next_mode),
    );

    let area_content: Element<'_, Message> = match state.active_area {
        Area::Dashboard => {
            area::dashboard::view(&state.dashboard, &state.project, &state.change.explorations)
                .map(Message::Dashboard)
        }
        Area::Change => area::change::view(&state.change, &state.project).map(Message::Change),
        Area::Caps => area::caps::view(&state.caps, &state.project).map(Message::Caps),
        Area::Codex => area::codex::view(&state.codex, &state.project).map(Message::Codex),
        Area::Settings => {
            area::settings::view(&state.settings, &state.config).map(Message::Settings)
        }
    };

    let segments = match state.active_area {
        Area::Dashboard => area::dashboard::breadcrumbs(),
        Area::Change => area::change::breadcrumbs(&state.change, &state.project),
        Area::Caps => area::caps::breadcrumbs(&state.caps),
        Area::Codex => area::codex::breadcrumbs(&state.codex, &state.project),
        Area::Settings => area::settings::breadcrumbs(),
    };
    let status_bar = widget::status_bar::view(segments);
    let status_divider = container(Space::new().width(Length::Fill))
        .height(1.0)
        .style(theme::divider);
    let area_with_status = column![
        container(area_content).height(Length::Fill),
        status_divider,
        status_bar,
    ]
    .height(Length::Fill);

    let sidebar_divider = container(Space::new().height(Length::Fill))
        .width(1.0)
        .style(theme::divider);
    let top_divider = container(Space::new().width(Length::Fill))
        .height(1.0)
        .style(theme::divider);
    let main_view = column![
        top_divider,
        row![sidebar, sidebar_divider, area_with_status].height(Length::Fill),
    ]
    .height(Length::Fill);

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
            watcher::watch_subscription(root.clone(), state.project.duckspec_root.clone())
                .map(Message::FileChanged),
        );
    }

    // Per-terminal PTY subscriptions. Keyed by the stable `instance_id` and
    // the per-tab `terminal.id` so each tab's shell survives scope renames
    // (e.g. exploration→change promotion) and tab reorders.
    let pty_cwd = state.project.project_root.clone();
    let push_pty = |ix: &interaction::InteractionState, subs: &mut Vec<Subscription<Message>>| {
        for tt in &ix.terminals {
            let key = format!("pty:ix:{}/term:{}", ix.instance_id, tt.id);
            subs.push(widget::terminal::pty_subscription(key, pty_cwd.clone()).map(tagged_pty));
        }
    };
    for ix in state.change.interactions.values() {
        push_pty(ix, &mut subs);
    }
    push_pty(&state.caps.interaction, &mut subs);
    push_pty(&state.codex.interaction, &mut subs);

    // Per-session agent subscriptions. Key format: `agent:ix:<instance_id>/<session_id>`.
    // Like PTYs, keyed by `instance_id` so in-flight agent streams survive renames.
    if let Some(root) = state.project.project_root.as_ref() {
        let push_scope = |ix: &interaction::InteractionState,
                          subs: &mut Vec<Subscription<Message>>| {
            for session in &ix.sessions {
                let key = format!("agent:ix:{}/{}", ix.instance_id, session.session.id);
                subs.push(agent::agent_subscription(key, root.clone()).map(tagged_agent));
            }
        };
        for ix in state.change.interactions.values() {
            push_scope(ix, &mut subs);
        }
        push_scope(&state.caps.interaction, &mut subs);
        push_scope(&state.codex.interaction, &mut subs);
    }

    // Global keyboard events.
    subs.push(event::listen_raw(handle_key_event));

    // Poll system dark/light mode.
    subs.push(theme_subscription());

    // Animation tick for the streaming indicator. Only subscribed when at
    // least one session is actively streaming, so idle chats don't wake
    // the render loop. Uses iced's built-in `time::every` so the timer runs
    // on iced's tokio runtime — the earlier handcrafted `tokio::time::sleep`
    // stream panicked silently under the default thread-pool backend.
    if any_session_streaming(state) {
        subs.push(
            iced::time::every(std::time::Duration::from_millis(
                widget::streaming_indicator::TICK_MS,
            ))
            .map(|_instant| Message::StreamTick),
        );
    }

    Subscription::batch(subs)
}

/// True if any session across all interaction panels is actively streaming.
fn any_session_streaming(state: &State) -> bool {
    let check =
        |ix: &interaction::InteractionState| ix.sessions.iter().any(|s| s.session.is_streaming);
    check(&state.caps.interaction)
        || check(&state.codex.interaction)
        || state.change.interactions.values().any(check)
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
    // Key shape: `pty:ix:{instance_id}/term:{terminal_id}`.
    let rest = key.strip_prefix("pty:ix:").unwrap_or(&key);
    let (ix_str, term_str) = rest.split_once("/term:").unwrap_or((rest, ""));
    let ix_id = ix_str.parse::<u64>().unwrap_or(0);
    let terminal_id = term_str.parse::<u64>().unwrap_or(0);
    Message::PtyEvent(ix_id, terminal_id, e)
}
fn tagged_agent((key, e): (String, agent::AgentEvent)) -> Message {
    // Strip the `agent:ix:` prefix; the remainder is `<instance_id>/<session_id>`.
    let routing_key = key.strip_prefix("agent:ix:").unwrap_or(&key).to_string();
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

fn handle_key_event(
    event: Event,
    status: event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        Event::Keyboard(keyboard::Event::ModifiersChanged(mods)) => {
            // Mirror modifier state into a process-wide cell so canvas widgets
            // (terminal, etc.) can react to cmd-held mouse moves and clicks.
            widget::terminal::set_current_modifiers(mods);
            None
        }
        Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            modifiers,
            text,
            ..
        }) => {
            widget::terminal::set_current_modifiers(modifiers);
            // Skip events already consumed by a focused widget (e.g. Enter typed
            // into the content editor). Otherwise the chat column would also
            // react to them. Escape is exempt: iced's `text_input` captures it to
            // clear focus, so without the exemption the file finder would need
            // two Escape presses to close.
            let is_escape = matches!(&key, keyboard::Key::Named(keyboard::key::Named::Escape));
            if !is_escape && matches!(status, event::Status::Captured) {
                return None;
            }
            Some(Message::KeyPress(
                key,
                modifiers,
                text.map(|s| s.to_string()),
            ))
        }
        _ => None,
    }
}
