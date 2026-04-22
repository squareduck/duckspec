//! Animated "thinking" indicator shown at the bottom of the chat transcript
//! while the agent is streaming a response.
//!
//! Renders three dots whose opacity pulses out of phase with each other.
//! Motion is driven by a global tick counter that the app increments via a
//! time subscription (see `bump_tick` / `tick`). The subscription is only
//! active while at least one session is streaming, so idle chats don't wake
//! the render loop.

use std::sync::atomic::{AtomicU64, Ordering};

use iced::advanced::Layout;
use iced::advanced::layout;
use iced::advanced::mouse;
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, Widget};
use iced::widget::{row, text};
use iced::{Border, Color, Element, Length, Rectangle, Size, Theme};

use crate::theme;

/// Monotonic tick counter. The app bumps this on a timer subscription so the
/// view layer can read a fresh value on every animation frame.
static TICK: AtomicU64 = AtomicU64::new(0);

/// Advance the animation by one frame. Called by the app from a time
/// subscription while any session is streaming.
pub fn bump_tick() {
    TICK.fetch_add(1, Ordering::Relaxed);
}

/// Recommended subscription cadence — 10 Hz is smooth enough for a pulsing
/// dot cycle and cheap enough to run during long streams.
pub const TICK_MS: u64 = 100;

/// Diameter of each animated dot, in pixels.
const DOT_DIAMETER: f32 = 5.0;

/// Three-dot pulsing indicator with an inline "esc to cancel" hint. Each dot's
/// opacity traces a sine wave, with the dots offset 120° apart so the pulse
/// travels left-to-right. `esc_count` is the current double-escape counter
/// used to adjust the hint text (first press switches to "esc to cancel").
pub fn view<'a, M: 'a>(esc_count: u8) -> Element<'a, M> {
    // 12 ticks (≈1.2s) per cycle.
    const CYCLE: u64 = 12;
    let tick = TICK.load(Ordering::Relaxed);

    let dot_color = |offset: u64| -> Color {
        let phase = (tick.wrapping_add(offset) % CYCLE) as f32 / CYCLE as f32;
        let pulse = (phase * std::f32::consts::TAU).sin() * 0.5 + 0.5;
        let alpha = 0.25 + 0.55 * pulse;
        Color {
            a: alpha,
            ..theme::text_muted()
        }
    };

    let dots = row![
        Dot::new(dot_color(0)),
        Dot::new(dot_color(CYCLE / 3)),
        Dot::new(dot_color(2 * CYCLE / 3)),
    ]
    .spacing(theme::SPACING_XS)
    .align_y(iced::Alignment::Center);

    let hint = if esc_count >= 2 {
        "cancelling\u{2026}"
    } else if esc_count == 1 {
        "esc to cancel"
    } else {
        "esc esc to cancel"
    };
    let hint_text = text(hint).size(theme::font_sm()).color(theme::text_muted());

    row![dots, hint_text]
        .spacing(theme::SPACING_SM)
        .align_y(iced::Alignment::Center)
        .into()
}

// ── Dot widget ────────────────────────────────────────────────────────────

/// Small filled circle, drawn as a `fill_quad` with a full half-diameter
/// border radius. Lets us position dots with exact pixel geometry instead of
/// relying on the bullet glyph's (font-dependent) bounding box.
struct Dot {
    color: Color,
}

impl Dot {
    fn new(color: Color) -> Self {
        Self { color }
    }
}

impl<M> Widget<M, Theme, iced::Renderer> for Dot {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(DOT_DIAMETER), Length::Fixed(DOT_DIAMETER))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(Size::new(DOT_DIAMETER, DOT_DIAMETER))
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        <iced::Renderer as renderer::Renderer>::fill_quad(
            renderer,
            renderer::Quad {
                bounds,
                border: Border {
                    radius: (bounds.width / 2.0).into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            self.color,
        );
    }
}

impl<'a, M: 'a> From<Dot> for Element<'a, M> {
    fn from(dot: Dot) -> Self {
        Element::new(dot)
    }
}
