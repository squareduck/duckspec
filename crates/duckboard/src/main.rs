//! duckboard — GUI for the duckspec framework, built with Iced 0.14.

use std::path::Path;

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
                                tabs.open(id, title, content);
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
            // Handle backlink clicks at this level.
            if let area::change::Message::BacklinkClicked(ref path) = msg {
                handle_backlink_click(state, path);
                return Task::none();
            }
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
            if let area::caps::Message::BacklinkClicked(ref path) = msg {
                handle_backlink_click(state, path);
                return Task::none();
            }
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
            if let area::codex::Message::BacklinkClicked(ref path) = msg {
                handle_backlink_click(state, path);
                return Task::none();
            }
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

            // Cmd+M: toggle edit mode on the active tab.
            if mods.command() && key == keyboard::Key::Character("m".into()) {
                match state.active_area {
                    Area::Change => {
                        let _ = update(
                            state,
                            Message::Change(area::change::Message::ToggleEditMode),
                        );
                    }
                    Area::Caps => {
                        let _ = update(
                            state,
                            Message::Caps(area::caps::Message::ToggleEditMode),
                        );
                    }
                    Area::Codex => {
                        let _ = update(
                            state,
                            Message::Codex(area::codex::Message::ToggleEditMode),
                        );
                    }
                    Area::Dashboard => {}
                }
                return Task::none();
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
            match &tab.view {
                tab_bar::TabView::Editor { .. } | tab_bar::TabView::Structural { .. } => {
                    if let Some(content) = state.project.read_artifact(&tab.id) {
                        tabs_refresh_single(tab, content);
                    }
                }
                tab_bar::TabView::Diff(_) => {}
            }
        }
    }
}

fn tabs_refresh_single(tab: &mut tab_bar::Tab, new_content: String) {
    match &mut tab.view {
        tab_bar::TabView::Editor { editor, .. } => {
            *editor = widget::text_edit::EditorState::new(&new_content);
        }
        tab_bar::TabView::Structural { source, .. } => {
            *source = new_content;
        }
        tab_bar::TabView::Diff(_) => {}
    }
}

/// Apply an editor action to the active tab's editor state.
pub fn handle_editor_action(
    tabs: &mut tab_bar::TabState,
    action: widget::text_edit::EditorAction,
) {
    use widget::text_edit::EditorAction;

    let tab = match tabs.active_tab_mut() {
        Some(t) => t,
        None => return,
    };
    let editor = match &mut tab.view {
        tab_bar::TabView::Editor { editor, .. } => editor,
        _ => return,
    };

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
        EditorAction::Scroll(dy) => {
            let max_scroll = (editor.line_count() as f32 * 20.0).max(0.0);
            editor.scroll_y = (editor.scroll_y + dy).clamp(0.0, max_scroll);
        }
        EditorAction::SaveRequested => {
            // TODO: write editor content back to disk.
            tracing::info!("save requested (not yet implemented)");
        }
    }
}

/// Refresh the VCS changed files list.
fn refresh_changed_files(state: &mut State) {
    if let Some(root) = &state.project.project_root {
        state.change.changed_files = vcs::changed_files(root);
    }
}

/// Handle a backlink click: open the referenced file in the active area's
/// tabs and (future) scroll to the referenced line.
fn handle_backlink_click(state: &mut State, backlink_path: &str) {
    // Backlink paths look like "tests/auth_test.rs:42" or "src/lib.rs:10".
    // They are relative to the project root.
    let (file_path, _line) = match backlink_path.rsplit_once(':') {
        Some((f, l)) => (f, l.parse::<usize>().ok()),
        None => (backlink_path, None),
    };

    let root = match &state.project.project_root {
        Some(r) => r.clone(),
        None => return,
    };

    let abs = root.join(file_path);
    let content = match std::fs::read_to_string(&abs) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("failed to open backlink target {}: {e}", abs.display());
            return;
        }
    };

    let id = format!("file:{file_path}");
    let title = Path::new(file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());

    // Open in the active area's tabs as a plain editor (non-artifact).
    let tabs = match state.active_area {
        Area::Change => &mut state.change.tabs,
        Area::Caps => &mut state.caps.tabs,
        Area::Codex => &mut state.codex.tabs,
        Area::Dashboard => {
            state.active_area = Area::Change;
            &mut state.change.tabs
        }
    };
    tabs.open(id.clone(), title, content);

    // Scroll to the referenced line if we have one.
    if let Some(line_num) = _line {
        if let Some(tab) = tabs.tabs.iter_mut().find(|t| t.id == id) {
            if let tab_bar::TabView::Editor { editor, .. } = &mut tab.view {
                let target = line_num.saturating_sub(1);
                editor.cursor = widget::text_edit::Pos::new(
                    target.min(editor.line_count().saturating_sub(1)),
                    0,
                );
                editor.scroll_to_cursor(600.0); // approximate viewport height
            }
        }
    }
}

// ── Artifact classification helper ──────────────────────────────────────────

/// Open a file as a tab, using structural view if it's a known artifact type.
/// Called from area update functions.
pub fn open_artifact_tab(
    tabs: &mut tab_bar::TabState,
    id: String,
    title: String,
    source: String,
    artifact_id: &str,
) {
    use duckpond::layout::{self, ArtifactKind};
    use duckpond::parse;
    use widget::structural_view::StructuralData;

    let path = std::path::Path::new(artifact_id);
    let kind = layout::classify(path);

    let structural = kind.and_then(|k| {
        let elements = parse::parse_elements(&source);
        match k {
            ArtifactKind::CapSpec | ArtifactKind::ChangeCapSpec => {
                parse::spec::parse_spec(&elements)
                    .ok()
                    .map(StructuralData::Spec)
            }
            ArtifactKind::CapDoc
            | ArtifactKind::ChangeCapDoc
            | ArtifactKind::Proposal
            | ArtifactKind::Design
            | ArtifactKind::Codex
            | ArtifactKind::Project => parse::doc::parse_document(&elements)
                .ok()
                .map(StructuralData::Document),
            ArtifactKind::Step => parse::step::parse_step(&elements)
                .ok()
                .map(StructuralData::Step),
            ArtifactKind::SpecDelta | ArtifactKind::DocDelta => {
                // Delta files could get structural view later; for now
                // fall through to editor.
                None
            }
        }
    });

    match structural {
        Some(data) => tabs.open_structural(id, title, source, data),
        None => tabs.open(id, title, source),
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
