pub mod caps;
pub mod change;
pub mod codex;
pub mod dashboard;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Dashboard,
    Change,
    Caps,
    Codex,
}

impl Area {
    pub const ALL: [Area; 4] = [
        Area::Dashboard,
        Area::Change,
        Area::Caps,
        Area::Codex,
    ];
}
