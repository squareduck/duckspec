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
    pub fn label(&self) -> &'static str {
        match self {
            Self::Dashboard => "D",
            Self::Change => "C",
            Self::Caps => "S",
            Self::Codex => "X",
        }
    }

    pub fn tooltip(&self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Change => "Changes",
            Self::Caps => "Capabilities",
            Self::Codex => "Codex",
        }
    }

    pub const ALL: [Area; 4] = [
        Area::Dashboard,
        Area::Change,
        Area::Caps,
        Area::Codex,
    ];
}
