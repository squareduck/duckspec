pub mod caps;
pub mod change;
pub mod codex;
pub mod dashboard;
pub mod ideas;
pub mod interaction;
pub mod settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Area {
    Dashboard,
    Ideas,
    Change,
    Caps,
    Codex,
    Settings,
}

impl Area {
    pub const NAV: [Area; 5] = [
        Area::Dashboard,
        Area::Change,
        Area::Ideas,
        Area::Caps,
        Area::Codex,
    ];
}
