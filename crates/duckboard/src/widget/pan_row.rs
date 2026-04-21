//! Selectable row whose background, hover, and click hit-area extend across
//! the full visible viewport — not just its natural content width.
//!
//! Required by sectioned lists wrapped in `horizontal_pan`: the row's layout
//! width must stay at its natural intrinsic width so the pan widget can
//! discover the column's max-row width during its measure pass, but the
//! highlight needs to look like a full-width sidebar row regardless of where
//! the user has panned. Reading `viewport` (the visible region in row coords)
//! lets us paint and hit-test across that visible window.
//!
//! Interaction model mirrors `iced::widget::button`: `Status::Active` /
//! `Hovered` / `Pressed` / `Disabled`, click on press-then-release.

use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{layout, mouse as adv_mouse, renderer as adv_renderer};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::widget::button;
use iced::{
    mouse, touch, Background, Color, Element, Event, Length, Padding,
    Rectangle, Size, Theme,
};

type StyleFn = fn(&Theme, button::Status) -> button::Style;

#[derive(Debug, Default)]
struct InternalState {
    is_pressed: bool,
}

pub struct PanRow<'a, M> {
    content: Element<'a, M>,
    on_press: Option<M>,
    style: StyleFn,
    padding: Padding,
}

impl<'a, M: Clone> PanRow<'a, M> {
    pub fn new(content: impl Into<Element<'a, M>>) -> Self {
        Self {
            content: content.into(),
            on_press: None,
            style: button::primary,
            padding: Padding::ZERO,
        }
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn on_press(mut self, msg: M) -> Self {
        self.on_press = Some(msg);
        self
    }

    pub fn style(mut self, style: StyleFn) -> Self {
        self.style = style;
        self
    }
}

/// The hit/highlight rectangle: full visible viewport width × row height,
/// anchored at the row's vertical position.
fn extended(row: Rectangle, viewport: &Rectangle) -> Rectangle {
    Rectangle {
        x: viewport.x,
        y: row.y,
        width: viewport.width,
        height: row.height,
    }
}

impl<'a, M: Clone> Widget<M, Theme, iced::Renderer> for PanRow<'a, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<InternalState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(InternalState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::padded(
            limits,
            Length::Shrink,
            Length::Shrink,
            self.padding,
            |limits| {
                self.content.as_widget_mut().layout(
                    &mut tree.children[0],
                    renderer,
                    limits,
                )
            },
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: adv_mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        let bounds = layout.bounds();
        let hit = extended(bounds, viewport);
        let internal = tree.state.downcast_mut::<InternalState>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if self.on_press.is_some() && cursor.is_over(hit) {
                    internal.is_pressed = true;
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. }) => {
                if let Some(on_press) = &self.on_press
                    && internal.is_pressed
                {
                    internal.is_pressed = false;
                    if cursor.is_over(hit) {
                        shell.publish(on_press.clone());
                    }
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Touch(touch::Event::FingerLost { .. }) => {
                internal.is_pressed = false;
            }
            // Cursor moved in or out of the extended hit area: redraw to
            // refresh the hover background. Cheap — list rows are short.
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                shell.request_redraw();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: adv_mouse::Cursor,
        viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let hit = extended(layout.bounds(), viewport);
        if self.on_press.is_some() && cursor.is_over(hit) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _defaults: &adv_renderer::Style,
        layout: Layout<'_>,
        cursor: adv_mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let hit = extended(bounds, viewport);
        let internal = tree.state.downcast_ref::<InternalState>();

        let status = if self.on_press.is_none() {
            button::Status::Disabled
        } else if cursor.is_over(hit) {
            if internal.is_pressed {
                button::Status::Pressed
            } else {
                button::Status::Hovered
            }
        } else {
            button::Status::Active
        };

        let style = (self.style)(theme, status);

        if style.background.is_some() || style.border.width > 0.0 {
            use iced::advanced::renderer::Renderer as _;
            renderer.fill_quad(
                adv_renderer::Quad {
                    bounds: hit,
                    border: style.border,
                    ..adv_renderer::Quad::default()
                },
                style
                    .background
                    .unwrap_or(Background::Color(Color::TRANSPARENT)),
            );
        }

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &adv_renderer::Style {
                text_color: style.text_color,
            },
            layout.children().next().unwrap(),
            cursor,
            viewport,
        );
    }
}

impl<'a, M: Clone + 'a> From<PanRow<'a, M>> for Element<'a, M> {
    fn from(r: PanRow<'a, M>) -> Self {
        Element::new(r)
    }
}
