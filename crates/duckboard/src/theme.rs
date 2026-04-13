//! Visual constants and style helpers for duckboard.

use iced::widget::{button, container};
use iced::{Border, Color, Theme};

// ── Palette (Catppuccin Macchiato) ───────────────────────────────────────────

const fn hex(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

// Base colors
pub const BG_BASE: Color = hex(0x24, 0x27, 0x3a);     // base
pub const BG_SURFACE: Color = hex(0x1e, 0x20, 0x30);   // mantle
pub const BG_ELEVATED: Color = hex(0x36, 0x3a, 0x4f);  // surface0
pub const BG_HOVER: Color = hex(0x49, 0x4d, 0x64);     // surface1

// Accent
pub const ACCENT: Color = hex(0x8a, 0xad, 0xf4);       // blue
pub const ACCENT_DIM: Color = hex(0x7d, 0xc4, 0xe4);   // sapphire

// Text
pub const TEXT_PRIMARY: Color = hex(0xca, 0xd3, 0xf5);  // text
pub const TEXT_SECONDARY: Color = hex(0xa5, 0xad, 0xce); // subtext0
pub const TEXT_MUTED: Color = hex(0x6e, 0x73, 0x8d);    // overlay0

// Border
pub const BORDER_COLOR: Color = hex(0x5b, 0x60, 0x78);  // surface2

// Diff backgrounds
pub const DIFF_ADDED_BG: Color = hex(0x1e, 0x30, 0x24);
pub const DIFF_REMOVED_BG: Color = hex(0x30, 0x1e, 0x22);
pub const DIFF_HUNK_BG: Color = hex(0x1e, 0x24, 0x30);

// Status
pub const SUCCESS: Color = hex(0xa6, 0xda, 0x95);       // green
pub const WARNING: Color = hex(0xee, 0xd4, 0x9f);       // yellow
pub const ERROR: Color = hex(0xed, 0x87, 0x96);          // red

// Extra palette (Catppuccin Macchiato)
pub const MAUVE: Color = hex(0xc6, 0xa0, 0xf6);
pub const PEACH: Color = hex(0xf5, 0xa9, 0x7f);
pub const TEAL: Color = hex(0x8b, 0xd5, 0xca);
pub const PINK: Color = hex(0xf5, 0xbd, 0xe6);
pub const LAVENDER: Color = hex(0xb7, 0xbd, 0xf8);

// ── Font sizes ──────────────────────────────────────────────────────────────

pub const FONT_SM: f32 = 11.0;
pub const FONT_MD: f32 = 13.0;
pub const FONT_LG: f32 = 16.0;

// ── Spacing ──────────────────────────────────────────────────────────────────

pub const SPACING_XS: f32 = 4.0;
pub const SPACING_SM: f32 = 8.0;
pub const SPACING_MD: f32 = 12.0;
pub const SPACING_LG: f32 = 16.0;
pub const SPACING_XL: f32 = 24.0;

// ── Dimensions ───────────────────────────────────────────────────────────────

pub const SIDEBAR_WIDTH: f32 = 48.0;
pub const LIST_COLUMN_WIDTH: f32 = 260.0;
pub const INTERACTION_COLUMN_WIDTH: f32 = 360.0;
pub const BORDER_RADIUS: f32 = 4.0;

// ── Custom theme ─────────────────────────────────────────────────────────────

pub fn app_theme() -> Theme {
    Theme::custom("duckboard".to_string(), iced::theme::Palette {
        background: BG_BASE,
        text: TEXT_PRIMARY,
        primary: ACCENT,
        success: SUCCESS,
        danger: ERROR,
        warning: WARNING,
    })
}

// ── Container styles ─────────────────────────────────────────────────────────

pub fn sidebar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(BG_BASE.into()),
        ..Default::default()
    }
}

pub fn surface(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(BG_SURFACE.into()),
        ..Default::default()
    }
}

pub fn elevated(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(BG_ELEVATED.into()),
        ..Default::default()
    }
}

pub fn panel(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(BG_SURFACE.into()),
        border: Border {
            color: BORDER_COLOR,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn divider(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(BORDER_COLOR.into()),
        ..Default::default()
    }
}

pub fn accent_bar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(ACCENT.into()),
        ..Default::default()
    }
}

// ── Button styles ────────────────────────────────────────────────────────────

pub fn nav_button_active(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(BG_ELEVATED.into()),
        text_color: ACCENT,
        border: Border {
            color: ACCENT,
            width: 1.0,
            radius: BORDER_RADIUS.into(),
        },
        ..Default::default()
    }
}

pub fn nav_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(BG_HOVER.into()),
        _ => None,
    };
    button::Style {
        background: bg,
        text_color: TEXT_SECONDARY,
        border: Border {
            radius: BORDER_RADIUS.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn list_item(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(BG_HOVER.into()),
        _ => None,
    };
    button::Style {
        background: bg,
        text_color: TEXT_PRIMARY,
        border: Border::default(),
        ..Default::default()
    }
}

pub fn list_item_active(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(BG_ELEVATED.into()),
        text_color: ACCENT,
        border: Border::default(),
        ..Default::default()
    }
}

pub fn tab_active(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(BG_SURFACE.into()),
        text_color: TEXT_PRIMARY,
        border: Border::default(),
        ..Default::default()
    }
}

pub fn tab_inactive(_theme: &Theme, status: button::Status) -> button::Style {
    let text = match status {
        button::Status::Hovered => TEXT_PRIMARY,
        _ => TEXT_MUTED,
    };
    button::Style {
        background: Some(BG_SURFACE.into()),
        text_color: text,
        border: Border::default(),
        ..Default::default()
    }
}

pub fn section_header(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(BG_HOVER.into()),
        _ => Some(BG_ELEVATED.into()),
    };
    button::Style {
        background: bg,
        text_color: TEXT_SECONDARY,
        border: Border {
            color: BORDER_COLOR,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn interaction_toggle(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(BG_HOVER.into()),
        _ => Some(BG_ELEVATED.into()),
    };
    button::Style {
        background: bg,
        text_color: TEXT_MUTED,
        border: Border::default(),
        ..Default::default()
    }
}

pub fn icon_button(_theme: &Theme, status: button::Status) -> button::Style {
    let color = match status {
        button::Status::Hovered => TEXT_PRIMARY,
        _ => TEXT_MUTED,
    };
    button::Style {
        background: None,
        text_color: color,
        border: Border::default(),
        ..Default::default()
    }
}

// ── Diff styles ─────────────────────────────────────────────────────────────

pub fn diff_added(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(DIFF_ADDED_BG.into()),
        ..Default::default()
    }
}

pub fn diff_removed(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(DIFF_REMOVED_BG.into()),
        ..Default::default()
    }
}

pub fn diff_hunk_header(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(DIFF_HUNK_BG.into()),
        ..Default::default()
    }
}

pub fn vcs_status_color(status: &crate::vcs::FileStatus) -> Color {
    match status {
        crate::vcs::FileStatus::Modified => WARNING,
        crate::vcs::FileStatus::Added => SUCCESS,
        crate::vcs::FileStatus::Deleted => ERROR,
    }
}

// ── Line number gutter ─────────────────────────────────────────────────────


