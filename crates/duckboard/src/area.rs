pub mod caps;
pub mod change;
pub mod codex;
pub mod dashboard;
pub mod interaction;
pub mod kanban;
pub mod settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Dashboard,
    Kanban,
    Change,
    Caps,
    Codex,
    Settings,
}

impl Area {
    pub const NAV: [Area; 5] = [
        Area::Dashboard,
        Area::Change,
        Area::Kanban,
        Area::Caps,
        Area::Codex,
    ];
}
