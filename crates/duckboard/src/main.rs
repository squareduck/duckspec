//! duckboard — GUI for the duckspec framework, built with Iced 0.14.

use iced::event;
use iced::keyboard;
use iced::widget::{row, stack};
use iced::{Element, Event, Subscription, Task};

mod area;
mod data;
mod theme;
mod vcs;
mod watcher;
mod widget;

use area::Area;
use data::ProjectData;

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

        Self {
            active_area: Area::Dashboard,
            project,
            dashboard: area::dashboard::State::default(),
            change,
            caps: area::caps::State::default(),
            codex: area::codex::State::default(),
            terminal: None,
            terminal_focused: false,
            file_finder: widget::file_finder::FileFinderState::default(),
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
                                match state.active_area {
                                    Area::Change => state.change.tabs.open(id, title, content),
                                    Area::Caps => state.caps.tabs.open(id, title, content),
                                    Area::Codex => state.codex.tabs.open(id, title, content),
                                    Area::Dashboard => {
                                        // Switch to change area for file viewing.
                                        state.active_area = Area::Change;
                                        state.change.tabs.open(id, title, content);
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
                                    state.change.tabs.refresh_content(&id, content.clone());
                                    state.caps.tabs.refresh_content(&id, content.clone());
                                    state.codex.tabs.refresh_content(&id, content);
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
            // Handle cross-area navigation before delegating.
            match &msg {
                area::dashboard::Message::ChangeClicked(name) => {
                    state.change.selected_change = Some(name.clone());
                    state.active_area = Area::Change;
                }
                area::dashboard::Message::ArchivedChangeClicked(name) => {
                    state.change.selected_change = Some(name.clone());
                    state.active_area = Area::Change;
                }
                area::dashboard::Message::NewChange => {
                    // Virtual change — placeholder for now.
                    tracing::info!("new change requested (not yet implemented)");
                }
            }
            area::dashboard::update(&mut state.dashboard, msg);
        }
        Message::Change(msg) => {
            if matches!(msg, area::change::Message::TerminalScroll) {
                let _ = update(state, Message::TerminalScroll);
                return Task::none();
            }
            let is_toggle = matches!(
                msg,
                area::change::Message::InteractionHandle(
                    widget::interaction_toggle::HandleMsg::Toggle
                )
            );
            area::change::update(&mut state.change, msg, &state.project);
            if is_toggle && state.change.interaction_visible && state.terminal.is_none() {
                let _ = update(state, Message::TerminalSpawn);
            }
            state.terminal_focused = state.change.interaction_visible;
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
            area::caps::update(&mut state.caps, msg, &state.project);
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
            area::codex::update(&mut state.codex, msg, &state.project);
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
        for tab in &mut tabs.tabs {
            if matches!(tab.content, widget::tab_bar::TabContent::Text(_)) {
                if let Some(content) = state.project.read_artifact(&tab.id) {
                    tab.content = widget::tab_bar::TabContent::Text(content);
                }
            }
        }
    }
}

/// Refresh the VCS changed files list.
fn refresh_changed_files(state: &mut State) {
    if let Some(root) = &state.project.project_root {
        state.change.changed_files = vcs::changed_files(root);
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

fn view(state: &State) -> Element<'_, Message> {
    let sidebar =
        widget::sidebar::view(&state.active_area, Message::AreaSelected, Message::Refresh);

    let term = state.terminal.as_ref();
    let area_content: Element<'_, Message> = match state.active_area {
        Area::Dashboard => {
            area::dashboard::view(&state.dashboard, &state.project).map(Message::Dashboard)
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

    let main_view = row![sidebar, area_content];

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
