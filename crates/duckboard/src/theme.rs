//! Visual constants and style helpers for duckboard.
//!
//! Supports dynamic dark/light mode switching.  The current mode is stored in a
//! global `AtomicBool`; every colour accessor reads it (single atomic load) and
//! returns the appropriate Catppuccin variant – Macchiato for dark, Latte for
//! light.

use std::sync::atomic::{AtomicBool, Ordering};

use iced::widget::{button, container, scrollable, svg};
use iced::{Background, Border, Color, Theme};

// ── Dark / light mode state ────────────────────────────────────────────────

static IS_DARK: AtomicBool = AtomicBool::new(true);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Dark,
    Light,
}

pub fn set_mode(mode: ColorMode) {
    IS_DARK.store(mode == ColorMode::Dark, Ordering::Relaxed);
}

pub fn mode() -> ColorMode {
    if IS_DARK.load(Ordering::Relaxed) {
        ColorMode::Dark
    } else {
        ColorMode::Light
    }
}

pub fn detect_mode() -> ColorMode {
    match dark_light::detect() {
        Ok(dark_light::Mode::Light) => ColorMode::Light,
        _ => ColorMode::Dark,
    }
}

// ── Colour helpers ─────────────────────────────────────────────────────────

const fn hex(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

fn pick(dark: Color, light: Color) -> Color {
    if IS_DARK.load(Ordering::Relaxed) { dark } else { light }
}

// ── Catppuccin Macchiato (dark) ────────────────────────────────────────────

#[allow(dead_code)]
mod macchiato {
    use super::{hex, Color};
    pub const BASE: Color = hex(0x24, 0x27, 0x3a);
    pub const MANTLE: Color = hex(0x1e, 0x20, 0x30);
    pub const SURFACE0: Color = hex(0x36, 0x3a, 0x4f);
    pub const SURFACE1: Color = hex(0x49, 0x4d, 0x64);
    pub const SURFACE2: Color = hex(0x5b, 0x60, 0x78);
    pub const OVERLAY0: Color = hex(0x6e, 0x73, 0x8d);
    pub const TEXT: Color = hex(0xca, 0xd3, 0xf5);
    pub const SUBTEXT0: Color = hex(0xa5, 0xad, 0xce);
    pub const BLUE: Color = hex(0x8a, 0xad, 0xf4);
    pub const SAPPHIRE: Color = hex(0x7d, 0xc4, 0xe4);
    pub const GREEN: Color = hex(0xa6, 0xda, 0x95);
    pub const YELLOW: Color = hex(0xee, 0xd4, 0x9f);
    pub const RED: Color = hex(0xed, 0x87, 0x96);
    pub const MAUVE: Color = hex(0xc6, 0xa0, 0xf6);
    pub const PEACH: Color = hex(0xf5, 0xa9, 0x7f);
    pub const TEAL: Color = hex(0x8b, 0xd5, 0xca);
    pub const PINK: Color = hex(0xf5, 0xbd, 0xe6);
    pub const LAVENDER: Color = hex(0xb7, 0xbd, 0xf8);
    pub const DIFF_ADDED_BG: Color = hex(0x1e, 0x30, 0x24);
    pub const DIFF_REMOVED_BG: Color = hex(0x30, 0x1e, 0x22);
    pub const DIFF_HUNK_BG: Color = hex(0x1e, 0x24, 0x30);
}

// ── Catppuccin Latte (light) ───────────────────────────────────────────────

#[allow(dead_code)]
mod latte {
    use super::{hex, Color};
    pub const BASE: Color = hex(0xef, 0xf1, 0xf5);
    pub const MANTLE: Color = hex(0xe6, 0xe9, 0xef);
    pub const SURFACE0: Color = hex(0xcc, 0xd0, 0xda);
    pub const SURFACE1: Color = hex(0xbc, 0xc0, 0xcc);
    pub const SURFACE2: Color = hex(0xac, 0xb0, 0xbe);
    pub const OVERLAY0: Color = hex(0x9c, 0xa0, 0xb0);
    pub const TEXT: Color = hex(0x4c, 0x4f, 0x69);
    pub const SUBTEXT0: Color = hex(0x6c, 0x6f, 0x85);
    pub const BLUE: Color = hex(0x1e, 0x66, 0xf5);
    pub const SAPPHIRE: Color = hex(0x20, 0x9f, 0xb5);
    pub const GREEN: Color = hex(0x40, 0xa0, 0x2b);
    pub const YELLOW: Color = hex(0xdf, 0x8e, 0x1d);
    pub const RED: Color = hex(0xd2, 0x0f, 0x39);
    pub const MAUVE: Color = hex(0x88, 0x39, 0xef);
    pub const PEACH: Color = hex(0xfe, 0x64, 0x0b);
    pub const TEAL: Color = hex(0x17, 0x92, 0x99);
    pub const PINK: Color = hex(0xea, 0x76, 0xcb);
    pub const LAVENDER: Color = hex(0x72, 0x87, 0xfd);
    pub const DIFF_ADDED_BG: Color = hex(0xd9, 0xf0, 0xd9);
    pub const DIFF_REMOVED_BG: Color = hex(0xf0, 0xd9, 0xdb);
    pub const DIFF_HUNK_BG: Color = hex(0xd9, 0xe2, 0xf0);
}

// ── Public colour accessors ────────────────────────────────────────────────
// Each is a single atomic load + branch – negligible cost.

pub fn bg_base() -> Color { pick(macchiato::BASE, latte::BASE) }
pub fn bg_surface() -> Color { pick(macchiato::MANTLE, latte::MANTLE) }
pub fn bg_elevated() -> Color { pick(macchiato::SURFACE0, latte::SURFACE0) }
pub fn bg_hover() -> Color { pick(macchiato::SURFACE1, latte::SURFACE1) }

/// Hover tint for list rows — halfway between the sidebar's surface and the
/// elevated header bg, so hover reads as a subtle step toward "lifted" without
/// matching the header shade.
pub fn bg_list_hover() -> Color {
    let a = bg_surface();
    let b = bg_elevated();
    Color {
        r: (a.r + b.r) * 0.5,
        g: (a.g + b.g) * 0.5,
        b: (a.b + b.b) * 0.5,
        a: 1.0,
    }
}

/// Background for collapsible section headers — a 70% mix toward
/// `bg_elevated`. The full elevated tone reads as too dark in the light
/// theme, but a flat halfway value collides with `bg_list_hover`, so the
/// header sits *above* the hover tint while still feeling softer than a
/// fully elevated surface.
pub fn bg_section_header() -> Color {
    let a = bg_surface();
    let b = bg_elevated();
    Color {
        r: a.r * 0.3 + b.r * 0.7,
        g: a.g * 0.3 + b.g * 0.7,
        b: a.b * 0.3 + b.b * 0.7,
        a: 1.0,
    }
}

/// Background for the chat transcript — a tone halfway between `bg_base` and
/// `bg_surface`, so the "paper-white" input (on `bg_base`) reads as a lifted
/// field above the transcript.
pub fn bg_chat_area() -> Color {
    let a = bg_base();
    let b = bg_surface();
    Color {
        r: (a.r + b.r) * 0.5,
        g: (a.g + b.g) * 0.5,
        b: (a.b + b.b) * 0.5,
        a: 1.0,
    }
}

// ── Chat message backgrounds ───────────────────────────────────────────────
// All sit in a narrow brightness band, distinguished by subtle colour tints.

pub fn chat_bg_user() -> Color {
    // Warm blue tint — lightest of the group.
    pick(hex(0x3c, 0x40, 0x5c), hex(0xd8, 0xdc, 0xeb))
}
pub fn chat_bg_assistant() -> Color {
    // Neutral — the baseline.
    pick(hex(0x33, 0x36, 0x4c), hex(0xe0, 0xe2, 0xea))
}
pub fn chat_bg_tool_use() -> Color {
    // Subtle cyan/sapphire tint.
    pick(hex(0x30, 0x3a, 0x4e), hex(0xdb, 0xe4, 0xea))
}
pub fn chat_bg_tool_result() -> Color {
    // Subtle green tint.
    pick(hex(0x30, 0x3a, 0x42), hex(0xdb, 0xe7, 0xdf))
}
pub fn chat_bg_system() -> Color {
    // Dimmest — falls back toward the editor base.
    pick(hex(0x2a, 0x2d, 0x40), hex(0xe6, 0xe8, 0xef))
}

pub fn accent() -> Color { pick(macchiato::BLUE, latte::BLUE) }
pub fn accent_dim() -> Color { pick(macchiato::SAPPHIRE, latte::SAPPHIRE) }

pub fn text_primary() -> Color { pick(macchiato::TEXT, latte::TEXT) }
pub fn text_secondary() -> Color { pick(macchiato::SUBTEXT0, latte::SUBTEXT0) }
pub fn text_muted() -> Color { pick(macchiato::OVERLAY0, latte::OVERLAY0) }

pub fn border_color() -> Color { pick(macchiato::SURFACE2, latte::SURFACE2) }

pub fn diff_added_bg() -> Color { pick(macchiato::DIFF_ADDED_BG, latte::DIFF_ADDED_BG) }
pub fn diff_removed_bg() -> Color { pick(macchiato::DIFF_REMOVED_BG, latte::DIFF_REMOVED_BG) }
pub fn diff_hunk_bg() -> Color { pick(macchiato::DIFF_HUNK_BG, latte::DIFF_HUNK_BG) }

pub fn success() -> Color { pick(macchiato::GREEN, latte::GREEN) }
pub fn warning() -> Color { pick(macchiato::YELLOW, latte::YELLOW) }
pub fn error() -> Color { pick(macchiato::RED, latte::RED) }

#[allow(dead_code)]
pub fn mauve() -> Color { pick(macchiato::MAUVE, latte::MAUVE) }
#[allow(dead_code)]
pub fn peach() -> Color { pick(macchiato::PEACH, latte::PEACH) }
#[allow(dead_code)]
pub fn teal() -> Color { pick(macchiato::TEAL, latte::TEAL) }
#[allow(dead_code)]
pub fn pink() -> Color { pick(macchiato::PINK, latte::PINK) }
#[allow(dead_code)]
pub fn lavender() -> Color { pick(macchiato::LAVENDER, latte::LAVENDER) }

// ── Font state ────────────────────────────────────────────────────────────

use std::sync::{OnceLock, RwLock};

struct FontState {
    #[allow(dead_code)]
    ui_font: iced::Font,
    ui_size: f32,
    content_font: iced::Font,
    content_size: f32,
    /// Pixel advance of a single monospace cell at `content_size`, cached from
    /// a cosmic-text layout of the current content font. Lazily measured.
    mono_cell: std::sync::OnceLock<(f32, f32)>,
}

impl Default for FontState {
    fn default() -> Self {
        Self {
            ui_font: iced::Font::DEFAULT,
            ui_size: 13.0,
            content_font: iced::Font::MONOSPACE,
            content_size: 13.0,
            mono_cell: std::sync::OnceLock::new(),
        }
    }
}

static FONTS: OnceLock<RwLock<FontState>> = OnceLock::new();

fn fonts() -> &'static RwLock<FontState> {
    FONTS.get_or_init(|| RwLock::new(FontState::default()))
}

pub fn set_fonts(config: &crate::config::Config) {
    let state = FontState {
        ui_font: crate::config::ui_font(config),
        ui_size: config.ui.font_size,
        content_font: crate::config::content_font(config),
        content_size: config.content.font_size,
        mono_cell: std::sync::OnceLock::new(),
    };
    *fonts().write().unwrap() = state;
}

#[allow(dead_code)]
pub fn ui_font() -> iced::Font {
    fonts().read().unwrap().ui_font
}

pub fn content_font() -> iced::Font {
    fonts().read().unwrap().content_font
}

pub fn ui_size() -> f32 {
    fonts().read().unwrap().ui_size
}

pub fn content_size() -> f32 {
    fonts().read().unwrap().content_size
}

/// Measure the monospace advance width and line height for the current content
/// font & size using cosmic-text (via iced's Paragraph). Cached per font state
/// so repeated calls are cheap. Falls back to heuristic ratios if the font
/// system isn't ready (shouldn't happen after the iced app starts).
fn mono_cell_measured() -> (f32, f32) {
    use iced::advanced::graphics::text::Paragraph;
    use iced::advanced::text::Paragraph as _;

    let guard = fonts().read().unwrap();
    *guard.mono_cell.get_or_init(|| {
        const SAMPLE_LEN: usize = 32;
        let sample: String = "M".repeat(SAMPLE_LEN);
        let text = iced::advanced::text::Text {
            content: sample.as_str(),
            bounds: iced::Size::INFINITE,
            size: iced::Pixels(guard.content_size),
            line_height: iced::widget::text::LineHeight::default(),
            font: guard.content_font,
            align_x: iced::advanced::text::Alignment::Left,
            align_y: iced::alignment::Vertical::Top,
            shaping: iced::widget::text::Shaping::Basic,
            wrapping: iced::widget::text::Wrapping::None,
        };
        let paragraph = Paragraph::with_text(text);
        let bounds = paragraph.min_bounds();
        let advance = bounds.width / SAMPLE_LEN as f32;
        if advance.is_finite() && advance > 0.0 {
            (advance, bounds.height.max(guard.content_size * 1.2))
        } else {
            (guard.content_size * 0.6, guard.content_size * 1.4)
        }
    })
}

pub fn content_cell_width() -> f32 {
    mono_cell_measured().0
}

pub fn content_cell_height() -> f32 {
    mono_cell_measured().1
}

// ── Font size helpers ─────────────────────────────────────────────────────
// Three tiers: small (config − 2), default (config), big (config + 2).

pub fn font_sm() -> f32 {
    (ui_size() - 2.0).max(6.0)
}

pub fn font_md() -> f32 {
    ui_size()
}

pub fn font_lg() -> f32 {
    ui_size() + 2.0
}

pub fn content_sm() -> f32 {
    (content_size() - 2.0).max(6.0)
}

pub fn content_lg() -> f32 {
    content_size() + 2.0
}


// ── Spacing ────────────────────────────────────────────────────────────────

pub const SPACING_XS: f32 = 4.0;
pub const SPACING_SM: f32 = 8.0;
pub const SPACING_MD: f32 = 12.0;
pub const SPACING_LG: f32 = 16.0;
pub const SPACING_XL: f32 = 24.0;

// ── Dimensions ─────────────────────────────────────────────────────────────

pub const SIDEBAR_WIDTH: f32 = 48.0;
pub const LIST_COLUMN_WIDTH: f32 = 260.0;
pub const INTERACTION_COLUMN_WIDTH: f32 = 360.0;
pub const BORDER_RADIUS: f32 = 4.0;

// ── Custom theme ───────────────────────────────────────────────────────────

pub fn app_theme() -> Theme {
    Theme::custom("duckboard".to_string(), iced::theme::Palette {
        background: bg_base(),
        text: text_primary(),
        primary: accent(),
        success: success(),
        danger: error(),
        warning: warning(),
    })
}

// ── SVG styles ────────────────────────────────────────────────────────────

pub fn svg_tint(color: Color) -> impl Fn(&iced::Theme, svg::Status) -> svg::Style {
    move |_theme, _status| svg::Style {
        color: Some(color),
    }
}

// ── Scrollable styles ─────────────────────────────────────────────────────

/// Thin, subtle scrollbar with reserved space so content doesn't overlap.
pub fn thin_scrollbar(_theme: &iced::Theme, _status: scrollable::Status) -> scrollable::Style {
    let scroller_color = text_muted();
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(scroller_color),
            border: Border {
                radius: 2.0.into(),
                ..Border::default()
            },
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        ..scrollable::default(_theme, _status)
    }
}

/// Scrollbar direction: thin vertical-only with reserved gutter space.
pub fn thin_scrollbar_direction() -> scrollable::Direction {
    scrollable::Direction::Vertical(
        scrollable::Scrollbar::new()
            .width(4)
            .scroller_width(4)
            .spacing(0),
    )
}

/// Scrollbar direction: thin horizontal-only with reserved gutter space.
pub fn thin_scrollbar_direction_horizontal() -> scrollable::Direction {
    scrollable::Direction::Horizontal(
        scrollable::Scrollbar::new()
            .width(4)
            .scroller_width(4)
            .spacing(0),
    )
}

// ── Container styles ───────────────────────────────────────────────────────

pub fn sidebar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_base().into()),
        ..Default::default()
    }
}

pub fn surface(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_surface().into()),
        ..Default::default()
    }
}

#[allow(dead_code)]
pub fn elevated(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_elevated().into()),
        ..Default::default()
    }
}

/// Background for the chat scroll area: a tone sitting between `bg_base` and
/// `bg_surface`, so the transcript reads as a slightly recessed surface
/// underneath the lifted input field.
pub fn chat_area(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_chat_area().into()),
        ..Default::default()
    }
}

/// Container style for the chat input: the lightest surface in the palette so
/// it reads as a lifted "paper" field above the chat transcript.
pub fn chat_input(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_base().into()),
        ..Default::default()
    }
}

/// Outer frame for tool-use / tool-result cards: 1px border + full radius,
/// no background. Inner header/body containers paint the surface colors.
pub fn chat_tool_card_frame(_theme: &Theme) -> container::Style {
    container::Style {
        border: Border {
            color: border_color(),
            width: 1.0,
            radius: BORDER_RADIUS.into(),
        },
        ..Default::default()
    }
}

/// Header surface of an open tool card — `bg_surface` (quiet, only one step
/// from the chat area) with the top corners rounded and bottom corners
/// square so it seats flush against the body below.
pub fn chat_tool_card_header_open(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_surface().into()),
        border: Border {
            radius: iced::border::Radius {
                top_left: BORDER_RADIUS,
                top_right: BORDER_RADIUS,
                bottom_right: 0.0,
                bottom_left: 0.0,
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Header surface of a collapsed tool card — same tint as the open header
/// but rounded on all four corners because there's no body beneath it.
pub fn chat_tool_card_header_alone(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_surface().into()),
        border: Border {
            radius: BORDER_RADIUS.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Body surface of an open tool card — matches the user bubble's "paper"
/// background (`bg_base`) with bottom corners rounded and top square so it
/// seats flush against the header above.
pub fn chat_tool_card_body(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_base().into()),
        border: Border {
            radius: iced::border::Radius {
                top_left: 0.0,
                top_right: 0.0,
                bottom_right: BORDER_RADIUS,
                bottom_left: BORDER_RADIUS,
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Card treatment for user messages in the chat — matches the input's
/// "paper" surface and framed border so prompts read as a distinct object
/// on the chat transcript, while assistant text flows unwrapped on the
/// chat background.
pub fn chat_user_card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_base().into()),
        border: Border {
            color: border_color(),
            width: 1.0,
            radius: BORDER_RADIUS.into(),
        },
        ..Default::default()
    }
}

pub fn audit_card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_surface().into()),
        border: Border {
            color: border_color(),
            width: 1.0,
            radius: BORDER_RADIUS.into(),
        },
        ..Default::default()
    }
}

pub fn divider(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(border_color().into()),
        ..Default::default()
    }
}

pub fn accent_bar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(accent().into()),
        ..Default::default()
    }
}

// ── Button styles ──────────────────────────────────────────────────────────

pub fn nav_button_active(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(bg_elevated().into()),
        text_color: accent(),
        border: Border {
            color: accent(),
            width: 1.0,
            radius: BORDER_RADIUS.into(),
        },
        ..Default::default()
    }
}

pub fn nav_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(bg_hover().into()),
        _ => None,
    };
    button::Style {
        background: bg,
        text_color: text_secondary(),
        border: Border {
            radius: BORDER_RADIUS.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn list_item(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(bg_list_hover().into()),
        _ => None,
    };
    button::Style {
        background: bg,
        text_color: text_primary(),
        border: Border::default(),
        ..Default::default()
    }
}

pub fn list_item_active(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(bg_list_hover().into()),
        _ => None,
    };
    button::Style {
        background: bg,
        text_color: accent(),
        border: Border::default(),
        ..Default::default()
    }
}

pub fn tab_active(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(bg_surface().into()),
        text_color: text_primary(),
        border: Border::default(),
        ..Default::default()
    }
}

pub fn tab_inactive(_theme: &Theme, status: button::Status) -> button::Style {
    let text = match status {
        button::Status::Hovered => text_primary(),
        _ => text_muted(),
    };
    button::Style {
        background: Some(bg_surface().into()),
        text_color: text,
        border: Border::default(),
        ..Default::default()
    }
}

pub fn section_header(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(bg_elevated().into()),
        _ => Some(bg_section_header().into()),
    };
    button::Style {
        background: bg,
        text_color: text_secondary(),
        border: Border::default(),
        ..Default::default()
    }
}

pub fn dashboard_action(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(bg_hover().into()),
        _ => None,
    };
    button::Style {
        background: bg,
        text_color: accent(),
        border: Border {
            color: border_color(),
            width: 1.0,
            radius: BORDER_RADIUS.into(),
        },
        ..Default::default()
    }
}

pub fn link_button(_theme: &Theme, status: button::Status) -> button::Style {
    let color = match status {
        button::Status::Hovered => text_primary(),
        _ => accent(),
    };
    button::Style {
        background: None,
        text_color: color,
        border: Border::default(),
        ..Default::default()
    }
}

pub fn icon_button(_theme: &Theme, status: button::Status) -> button::Style {
    let color = match status {
        button::Status::Hovered => text_primary(),
        _ => text_muted(),
    };
    button::Style {
        background: None,
        text_color: color,
        border: Border::default(),
        ..Default::default()
    }
}

/// Compact button for the session bar (`+`, `Clear`).
pub fn session_bar_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(bg_hover().into()),
        _ => Some(bg_surface().into()),
    };
    button::Style {
        background: bg,
        text_color: text_secondary(),
        border: Border {
            color: border_color(),
            width: 1.0,
            radius: BORDER_RADIUS.into(),
        },
        ..Default::default()
    }
}

// ── VCS helpers ────────────────────────────────────────────────────────────

pub fn vcs_status_color(status: &crate::vcs::FileStatus) -> Color {
    match status {
        crate::vcs::FileStatus::Modified => warning(),
        crate::vcs::FileStatus::Added => success(),
        crate::vcs::FileStatus::Deleted => error(),
    }
}
