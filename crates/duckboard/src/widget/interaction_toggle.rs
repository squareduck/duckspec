//! Draggable divider strip that toggles or resizes the interaction column.
//!
//! Click (press + release without significant horizontal movement) → toggle.
//! Drag horizontally → resize the interaction column.

use iced::advanced::layout;
use iced::advanced::mouse as adv_mouse;
use iced::advanced::renderer;
use iced::advanced::svg;
use iced::advanced::widget::{self, Tree, Widget};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::mouse;
use iced::{Border, Element, Event, Length, Rectangle, Size, Theme};

use crate::theme;

const HANDLE_WIDTH: f32 = 16.0;
const CHEVRON_SIZE: f32 = 12.0;
/// Horizontal inset centers the chevron in the strip.
const CHEVRON_INSET_X: f32 = (HANDLE_WIDTH - CHEVRON_SIZE) / 2.0;
/// Top inset sits the chevron a bit further down so it has some breathing
/// room from the window chrome above.
const CHEVRON_INSET_Y: f32 = 6.0;
const DRAG_THRESHOLD: f32 = 4.0;

const ICON_CHEVRON_RIGHT: &[u8] =
    include_bytes!("../../assets/icon_chevron_right.svg");
const ICON_CHEVRON_LEFT: &[u8] = include_bytes!("../../assets/icon_chevron_left.svg");

/// Messages produced by the divider handle.
#[derive(Debug, Clone)]
pub enum HandleMsg {
    Toggle,
    /// Set the panel width to this absolute value.
    SetWidth(f32),
}

struct DragState {
    start_x: f32,
    base_width: f32,
    dragging: bool,
}

#[derive(Default)]
struct HandleState {
    drag: Option<DragState>,
    hovered: bool,
}

const MIN_PANEL_WIDTH: f32 = 200.0;
const MAX_PANEL_WIDTH: f32 = 800.0;

/// The divider handle widget.
pub struct InteractionHandle<'a, M> {
    expanded: bool,
    current_width: f32,
    on_event: Box<dyn Fn(HandleMsg) -> M + 'a>,
}

impl<'a, M> InteractionHandle<'a, M> {
    pub fn new(
        expanded: bool,
        current_width: f32,
        on_event: impl Fn(HandleMsg) -> M + 'a,
    ) -> Self {
        Self {
            expanded,
            current_width,
            on_event: Box::new(on_event),
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
        if let Event::Mouse(
            mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft,
        ) = event
        {
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
                        // Negative dx (drag left) = grow panel.
                        let new_width = (state.base_width - dx)
                            .clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH);
                        shell.publish((self.on_event)(HandleMsg::SetWidth(new_width)));
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(state) = widget_state.drag.take()
                    && !state.dragging
                {
                    shell.publish((self.on_event)(HandleMsg::Toggle));
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
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::ResizingHorizontally
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
        let bg = if hovered { theme::bg_surface() } else { theme::bg_base() };
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

        // Chevron icon — top-aligned, with equal padding on top / left /
        // right so the icon's optical whitespace matches the strip width.
        let chevron_bytes = if self.expanded {
            ICON_CHEVRON_RIGHT
        } else {
            ICON_CHEVRON_LEFT
        };
        let chevron_bounds = Rectangle {
            x: bounds.x + CHEVRON_INSET_X,
            y: bounds.y + CHEVRON_INSET_Y,
            width: CHEVRON_SIZE,
            height: CHEVRON_SIZE,
        };
        <iced::Renderer as svg::Renderer>::draw_svg(
            renderer,
            svg::Svg::new(svg::Handle::from_memory(chevron_bytes))
                .color(theme::text_muted()),
            chevron_bounds,
            bounds,
        );
    }
}

impl<'a, M: Clone + 'a> From<InteractionHandle<'a, M>> for Element<'a, M> {
    fn from(handle: InteractionHandle<'a, M>) -> Self {
        Self::new(handle)
    }
}

/// Convenience constructor.
pub fn view<'a, M: Clone + 'a>(
    expanded: bool,
    current_width: f32,
    on_event: impl Fn(HandleMsg) -> M + 'a,
) -> Element<'a, M> {
    InteractionHandle::new(expanded, current_width, on_event).into()
}
