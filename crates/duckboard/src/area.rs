pub mod caps;
pub mod change;
pub mod codex;
pub mod dashboard;
pub mod interaction;
pub mod settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Dashboard,
    Change,
    Caps,
    Codex,
    Settings,
}

impl Area {
    pub const NAV: [Area; 4] = [
        Area::Dashboard,
        Area::Change,
        Area::Caps,
        Area::Codex,
    ];
}
