//! duckboard — GUI for the duckspec framework, built with Iced 0.14.

use iced::event;
use iced::keyboard;
use iced::widget::row;
use iced::{Element, Event, Subscription};

mod area;
mod data;
mod theme;
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
        Self {
            active_area: Area::Dashboard,
            project,
            dashboard: area::dashboard::State::default(),
            change: area::change::State::default(),
            caps: area::caps::State::default(),
            codex: area::codex::State::default(),
            terminal: None,
            terminal_focused: false,
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
    // Terminal
    TerminalSpawn,
    TerminalScroll,
    PtyEvent(widget::terminal::PtyEvent),
    TerminalKeyPress(keyboard::Key, keyboard::Modifiers, Option<String>),
}

// ── Update ───────────────────────────────────────────────────────────────────

fn update(state: &mut State, message: Message) {
    match message {
        Message::AreaSelected(area) => {
            state.active_area = area;
        }
        Message::Refresh => {
            state.project.reload();
            tracing::info!("project reloaded");
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
                update(state, Message::TerminalScroll);
                return;
            }
            let is_toggle = matches!(
                msg,
                area::change::Message::InteractionHandle(
                    widget::interaction_toggle::HandleMsg::Toggle
                )
            );
            area::change::update(&mut state.change, msg, &state.project);
            if is_toggle && state.change.interaction_visible && state.terminal.is_none() {
                update(state, Message::TerminalSpawn);
            }
            state.terminal_focused = state.change.interaction_visible;
        }
        Message::Caps(msg) => {
            if matches!(msg, area::caps::Message::TerminalScroll) {
                update(state, Message::TerminalScroll);
                return;
            }
            let is_toggle = matches!(
                msg,
                area::caps::Message::InteractionHandle(
                    widget::interaction_toggle::HandleMsg::Toggle
                )
            );
            area::caps::update(&mut state.caps, msg, &state.project);
            if is_toggle && state.caps.interaction_visible && state.terminal.is_none() {
                update(state, Message::TerminalSpawn);
            }
            state.terminal_focused = state.caps.interaction_visible;
        }
        Message::Codex(msg) => {
            if matches!(msg, area::codex::Message::TerminalScroll) {
                update(state, Message::TerminalScroll);
                return;
            }
            let is_toggle = matches!(
                msg,
                area::codex::Message::InteractionHandle(
                    widget::interaction_toggle::HandleMsg::Toggle
                )
            );
            area::codex::update(&mut state.codex, msg, &state.project);
            if is_toggle && state.codex.interaction_visible && state.terminal.is_none() {
                update(state, Message::TerminalSpawn);
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
        Message::TerminalKeyPress(key, mods, text) => {
            if let Some(ref mut ts) = state.terminal {
                ts.write_key(key, mods, text.as_deref());
            }
        }
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

    row![sidebar, area_content].into()
}

// ── Main ─────────────────────────────────────────────────────────────────────

// ── Subscription ────────────────────────────────────────────────────────────

fn subscription(state: &State) -> Subscription<Message> {
    let mut subs = vec![];

    // PTY I/O subscription: active when a terminal exists.
    if state.terminal.is_some() {
        subs.push(
            widget::terminal::pty_subscription().map(Message::PtyEvent),
        );
    }

    // Keyboard capture: active when terminal is focused.
    if state.terminal_focused && state.terminal.is_some() {
        subs.push(event::listen_raw(|event, _status, _window| {
            if let Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                text,
                ..
            }) = event
            {
                Some(Message::TerminalKeyPress(
                    key,
                    modifiers,
                    text.map(|s| s.to_string()),
                ))
            } else {
                None
            }
        }));
    }

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
