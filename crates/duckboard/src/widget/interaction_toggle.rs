//! Draggable divider strip that toggles or resizes the interaction column.
//!
//! Click (press + release without significant horizontal movement) → toggle.
//! Drag horizontally → resize the interaction column.

use iced::advanced::layout;
use iced::advanced::mouse as adv_mouse;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Tree, Widget};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::mouse;
use iced::{alignment, Border, Element, Event, Length, Rectangle, Size, Theme};

use crate::theme;

const HANDLE_WIDTH: f32 = 16.0;
const DRAG_THRESHOLD: f32 = 4.0;

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

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(Option::<DragState>::None)
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
        let drag = tree.state.downcast_mut::<Option<DragState>>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.is_over(bounds) {
                    let pos = cursor.position().unwrap();
                    *drag = Some(DragState {
                        start_x: pos.x,
                        base_width: self.current_width,
                        dragging: false,
                    });
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(state) = drag {
                    let dx = position.x - state.start_x;
                    if !state.dragging && dx.abs() > DRAG_THRESHOLD {
                        state.dragging = true;
                        // When panel is visible, handle is left of panel.
                        // Dragging left = growing panel. Use the total
                        // displacement from start as the resize delta.
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
                if let Some(state) = drag.take() {
                    if !state.dragging {
                        shell.publish((self.on_event)(HandleMsg::Toggle));
                    }
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
        let drag = tree.state.downcast_ref::<Option<DragState>>();
        if drag.is_some() {
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
        _tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: adv_mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let hovered = cursor.is_over(bounds);

        let bg = if hovered { theme::BG_HOVER } else { theme::BG_ELEVATED };
        renderer::Renderer::fill_quad(
            renderer,
            renderer::Quad {
                bounds,
                border: Border::default(),
                ..renderer::Quad::default()
            },
            bg,
        );

        // Arrow indicator — top of the strip, centered horizontally.
        let arrow: &str = if self.expanded { "\u{25b8}" } else { "\u{25c2}" };
        let arrow_size = 16.0;
        let arrow_y = bounds.y + theme::SPACING_SM;
        // Center the glyph by placing it at the midpoint minus half glyph width.
        let glyph_width = arrow_size * 0.6;
        let arrow_x = bounds.x + (bounds.width - glyph_width) / 2.0;

        use iced::advanced::text::Renderer as TextRenderer;
        renderer.fill_text(
            iced::advanced::Text {
                content: arrow.to_string(),
                bounds: Size::new(arrow_size, arrow_size),
                size: iced::Pixels(arrow_size),
                line_height: iced::advanced::text::LineHeight::Absolute(iced::Pixels(arrow_size)),
                font: iced::Font::default(),
                align_x: alignment::Horizontal::Left.into(),
                align_y: alignment::Vertical::Top,
                shaping: iced::advanced::text::Shaping::Basic,
                wrapping: iced::advanced::text::Wrapping::None,
            },
            iced::Point::new(arrow_x, arrow_y),
            theme::TEXT_MUTED,
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
