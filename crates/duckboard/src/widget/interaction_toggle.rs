//! Draggable divider strip between the content and interaction columns.
//!
//! The strip behaves like a sliding door with three resting points and three
//! affordances stacked top-to-bottom:
//!
//! - **Top chevron** — toggles the chat/terminal panel closed ↔ open (the
//!   door's near end). Click only.
//! - **Middle grip (`⋮`)** — drag to resize the panel continuously.
//! - **Bottom chevron** — snaps the door fully open (content collapses, panel
//!   fills) and back. Click only; flips `‹` → `›` once content is collapsed.

use iced::advanced::layout;
use iced::advanced::mouse as adv_mouse;
use iced::advanced::renderer;
use iced::advanced::svg;
use iced::advanced::widget::{self, Tree, Widget};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::mouse;
use iced::{Border, Color, Element, Event, Length, Rectangle, Size, Theme};

use crate::theme;

/// Width of the door strip between content and interaction columns.
pub const HANDLE_WIDTH: f32 = 16.0;
const CHEVRON_SIZE: f32 = 12.0;
/// Horizontal inset centers a chevron in the strip.
const CHEVRON_INSET_X: f32 = (HANDLE_WIDTH - CHEVRON_SIZE) / 2.0;
/// Vertical breathing room between a chevron and the nearest strip edge.
const CHEVRON_INSET_Y: f32 = 6.0;
/// Height of the click-only button zones at the top and bottom of the strip.
const BUTTON_ZONE_H: f32 = CHEVRON_SIZE + CHEVRON_INSET_Y * 2.0;
const DRAG_THRESHOLD: f32 = 4.0;

/// Grip dots drawn in the middle zone to signal "drag me".
const GRIP_DOT: f32 = 3.0;
const GRIP_DOT_GAP: f32 = 3.0;
const GRIP_DOT_COUNT: usize = 3;

/// Minimum interaction-column width when content is shown (equal split and drag).
pub const MIN_PANEL_WIDTH: f32 = 200.0;

const ICON_CHEVRON_RIGHT: &[u8] = include_bytes!("../../assets/icon_chevron_right.svg");
const ICON_CHEVRON_LEFT: &[u8] = include_bytes!("../../assets/icon_chevron_left.svg");

/// Messages produced by the divider handle.
#[derive(Debug, Clone)]
pub enum HandleMsg {
    /// Toggle the panel closed ↔ open (top chevron / middle click).
    Toggle,
    /// Set the panel width to this absolute value. Implies content is shown.
    SetWidth(f32),
    /// Collapse the content column (`true`, panel fills) or restore it
    /// (`false`, panel returns to its remembered width).
    SetCollapsed(bool),
}

#[derive(Clone, Copy, PartialEq)]
enum Zone {
    Top,
    Middle,
    Bottom,
}

struct DragState {
    start_x: f32,
    base_width: f32,
    zone: Zone,
    dragging: bool,
}

#[derive(Default)]
struct HandleState {
    drag: Option<DragState>,
    hovered: bool,
}

/// The divider handle widget.
pub struct InteractionHandle<'a, M> {
    /// Whether the panel is currently open.
    expanded: bool,
    /// Whether the content column is collapsed (panel filling its space).
    collapsed: bool,
    current_width: f32,
    /// Upper clamp for grip drag — free content↔chat space (not a fixed max).
    max_width: f32,
    on_event: Box<dyn Fn(HandleMsg) -> M + 'a>,
}

impl<'a, M> InteractionHandle<'a, M> {
    pub fn new(
        expanded: bool,
        collapsed: bool,
        current_width: f32,
        max_width: f32,
        on_event: impl Fn(HandleMsg) -> M + 'a,
    ) -> Self {
        Self {
            expanded,
            collapsed,
            current_width,
            max_width,
            on_event: Box::new(on_event),
        }
    }

    /// Classify a cursor y-coordinate (absolute) into a strip zone.
    fn zone_at(bounds: Rectangle, y: f32) -> Zone {
        if y <= bounds.y + BUTTON_ZONE_H {
            Zone::Top
        } else if y >= bounds.y + bounds.height - BUTTON_ZONE_H {
            Zone::Bottom
        } else {
            Zone::Middle
        }
    }
}

impl<'a, M: Clone> Widget<M, Theme, iced::Renderer> for InteractionHandle<'a, M> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(HANDLE_WIDTH), Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(HANDLE_WIDTH).height(Length::Fill);
        layout::Node::new(limits.resolve(HANDLE_WIDTH, Length::Fill, Size::ZERO))
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<HandleState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(HandleState::default())
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: adv_mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let widget_state = tree.state.downcast_mut::<HandleState>();

        // Track hover in internal state so the visual reliably reverts when
        // the cursor leaves. Relying on `cursor.is_over(bounds)` in `draw`
        // alone caused stuck-hover when iced skipped a redraw between the
        // last over-the-widget CursorMoved and the next one off-widget.
        if let Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft) = event {
            let now_hovered = cursor.is_over(bounds);
            if widget_state.hovered != now_hovered {
                widget_state.hovered = now_hovered;
                shell.request_redraw();
            }
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.is_over(bounds) {
                    let pos = cursor.position().unwrap();
                    widget_state.drag = Some(DragState {
                        start_x: pos.x,
                        base_width: self.current_width,
                        zone: Self::zone_at(bounds, pos.y),
                        dragging: false,
                    });
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(state) = widget_state.drag.as_mut() {
                    let dx = position.x - state.start_x;
                    if !state.dragging && dx.abs() > DRAG_THRESHOLD {
                        state.dragging = true;
                    }
                    if state.dragging {
                        // Negative dx (drag left) = grow panel. Min panel width
                        // and free-space max — collapsing content is bottom
                        // chevron only, never a drag past free space.
                        let max = self.max_width.max(MIN_PANEL_WIDTH);
                        let new_width = (state.base_width - dx).clamp(MIN_PANEL_WIDTH, max);
                        shell.publish((self.on_event)(HandleMsg::SetWidth(new_width)));
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(state) = widget_state.drag.take()
                    && !state.dragging
                {
                    // A click (no drag): the zone decides the action.
                    let msg = match state.zone {
                        Zone::Bottom => HandleMsg::SetCollapsed(!self.collapsed),
                        Zone::Top | Zone::Middle => HandleMsg::Toggle,
                    };
                    shell.publish((self.on_event)(msg));
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: adv_mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let widget_state = tree.state.downcast_ref::<HandleState>();
        if widget_state.drag.is_some() {
            return mouse::Interaction::ResizingHorizontally;
        }
        let bounds = layout.bounds();
        if let Some(pos) = cursor.position_over(bounds) {
            match Self::zone_at(bounds, pos.y) {
                Zone::Middle => mouse::Interaction::ResizingHorizontally,
                Zone::Top | Zone::Bottom => mouse::Interaction::Pointer,
            }
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: adv_mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let hovered = tree.state.downcast_ref::<HandleState>().hovered;

        // Background — lighter than the surrounding surface so the strip
        // reads as a lifted divider. Hover bumps it one step.
        let bg = if hovered {
            theme::bg_surface()
        } else {
            theme::bg_base()
        };
        renderer::Renderer::fill_quad(
            renderer,
            renderer::Quad {
                bounds,
                border: Border::default(),
                ..renderer::Quad::default()
            },
            bg,
        );

        // Vertical separators on both edges so the drag strip reads as a
        // distinct, hit-testable zone between the main content and the chat.
        let sep_color = theme::border_color();
        renderer::Renderer::fill_quad(
            renderer,
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x,
                    y: bounds.y,
                    width: 1.0,
                    height: bounds.height,
                },
                border: Border::default(),
                ..renderer::Quad::default()
            },
            sep_color,
        );
        renderer::Renderer::fill_quad(
            renderer,
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x + bounds.width - 1.0,
                    y: bounds.y,
                    width: 1.0,
                    height: bounds.height,
                },
                border: Border::default(),
                ..renderer::Quad::default()
            },
            sep_color,
        );

        let icon_color = theme::text_muted();

        // Top chevron — toggles the panel. Points "in" (right) when the panel
        // is open (click to close), "out" (left) when closed (click to open).
        let top_icon = if self.expanded {
            ICON_CHEVRON_RIGHT
        } else {
            ICON_CHEVRON_LEFT
        };
        draw_chevron(
            renderer,
            top_icon,
            icon_color,
            bounds,
            bounds.y + CHEVRON_INSET_Y,
        );

        // Middle grip — vertical "⋮" dots signalling the strip is draggable.
        let total_h =
            GRIP_DOT_COUNT as f32 * GRIP_DOT + (GRIP_DOT_COUNT as f32 - 1.0) * GRIP_DOT_GAP;
        let dot_x = bounds.x + (HANDLE_WIDTH - GRIP_DOT) / 2.0;
        let mut dot_y = bounds.y + (bounds.height - total_h) / 2.0;
        for _ in 0..GRIP_DOT_COUNT {
            renderer::Renderer::fill_quad(
                renderer,
                renderer::Quad {
                    bounds: Rectangle {
                        x: dot_x,
                        y: dot_y,
                        width: GRIP_DOT,
                        height: GRIP_DOT,
                    },
                    border: Border {
                        radius: (GRIP_DOT / 2.0).into(),
                        ..Border::default()
                    },
                    ..renderer::Quad::default()
                },
                icon_color,
            );
            dot_y += GRIP_DOT + GRIP_DOT_GAP;
        }

        // Bottom chevron — snaps the door fully open. Points "out" (left,
        // `‹`) to expand fully; flips "in" (right, `›`) once content is
        // collapsed, to give the content back.
        let bottom_icon = if self.collapsed {
            ICON_CHEVRON_RIGHT
        } else {
            ICON_CHEVRON_LEFT
        };
        draw_chevron(
            renderer,
            bottom_icon,
            icon_color,
            bounds,
            bounds.y + bounds.height - CHEVRON_INSET_Y - CHEVRON_SIZE,
        );
    }
}

/// Draw a chevron SVG centered horizontally at the given top y.
fn draw_chevron(
    renderer: &mut iced::Renderer,
    bytes: &'static [u8],
    color: Color,
    bounds: Rectangle,
    top_y: f32,
) {
    let icon_bounds = Rectangle {
        x: bounds.x + CHEVRON_INSET_X,
        y: top_y,
        width: CHEVRON_SIZE,
        height: CHEVRON_SIZE,
    };
    <iced::Renderer as svg::Renderer>::draw_svg(
        renderer,
        svg::Svg::new(svg::Handle::from_memory(bytes)).color(color),
        icon_bounds,
        bounds,
    );
}

impl<'a, M: Clone + 'a> From<InteractionHandle<'a, M>> for Element<'a, M> {
    fn from(handle: InteractionHandle<'a, M>) -> Self {
        Self::new(handle)
    }
}

/// Convenience constructor.
pub fn view<'a, M: Clone + 'a>(
    expanded: bool,
    collapsed: bool,
    current_width: f32,
    max_width: f32,
    on_event: impl Fn(HandleMsg) -> M + 'a,
) -> Element<'a, M> {
    InteractionHandle::new(expanded, collapsed, current_width, max_width, on_event).into()
}
