//! duckboard — GUI for the duckspec framework, built with Iced 0.14.

use iced::widget::row;
use iced::Element;

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
            area::change::update(&mut state.change, msg, &state.project);
        }
        Message::Caps(msg) => {
            area::caps::update(&mut state.caps, msg, &state.project);
        }
        Message::Codex(msg) => {
            area::codex::update(&mut state.codex, msg, &state.project);
        }
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

fn view(state: &State) -> Element<'_, Message> {
    let sidebar =
        widget::sidebar::view(&state.active_area, Message::AreaSelected, Message::Refresh);

    let area_content: Element<'_, Message> = match state.active_area {
        Area::Dashboard => {
            area::dashboard::view(&state.dashboard, &state.project).map(Message::Dashboard)
        }
        Area::Change => {
            area::change::view(&state.change, &state.project).map(Message::Change)
        }
        Area::Caps => area::caps::view(&state.caps, &state.project).map(Message::Caps),
        Area::Codex => {
            area::codex::view(&state.codex, &state.project).map(Message::Codex)
        }
    };

    row![sidebar, area_content].into()
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();
    tracing::info!("duckboard starting");

    iced::application(State::new, update, view)
        .title("duckboard")
        .theme(theme_fn)
        .window_size((1200.0, 800.0))
        .run()
}

fn theme_fn(_state: &State) -> iced::Theme {
    theme::app_theme()
}
