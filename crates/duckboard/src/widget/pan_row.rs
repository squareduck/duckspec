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

use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::advanced::{layout, mouse as adv_mouse, renderer as adv_renderer};
use iced::widget::button;
use iced::{
    Background, Color, Element, Event, Length, Padding, Rectangle, Size, Theme, mouse, touch,
};

type StyleFn = fn(&Theme, button::Status) -> button::Style;

#[derive(Debug, Default)]
struct InternalState {
    is_pressed: bool,
    is_hovered: bool,
}

pub struct PanRow<'a, M> {
    content: Element<'a, M>,
    on_press: Option<M>,
    on_hover_enter: Option<M>,
    on_hover_exit: Option<M>,
    style: StyleFn,
    padding: Padding,
}

impl<'a, M: Clone> PanRow<'a, M> {
    pub fn new(content: impl Into<Element<'a, M>>) -> Self {
        Self {
            content: content.into(),
            on_press: None,
            on_hover_enter: None,
            on_hover_exit: None,
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

    /// Fire `enter` when the cursor enters the row's viewport-wide hit
    /// area and `exit` when it leaves. Uses `PanRow`'s own hover state so
    /// detection is stable across tree-diff changes in the row's content
    /// (e.g. swapping an icon for a close button on hover).
    pub fn on_hover(mut self, enter: M, exit: M) -> Self {
        self.on_hover_enter = Some(enter);
        self.on_hover_exit = Some(exit);
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
        let pad = self.padding;
        let content_node = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &limits.shrink(pad),
        );
        let content_size = content_node.size();
        let content_node = content_node.move_to((pad.left, pad.top));
        let row_size = content_size.expand(pad);

        layout::Node::with_children(row_size, vec![content_node])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        let content_layout = layout.children().next().unwrap();
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            content_layout,
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
        let bounds = layout.bounds();
        let content_layout = layout.children().next().unwrap();

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            content_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        let hit = extended(bounds, viewport);
        let internal = tree.state.downcast_mut::<InternalState>();

        // Hover-transition detection driven by cursor position vs. hit
        // area. Using `PanRow`'s own tree state (not `MouseArea`'s) keeps
        // this stable when the content swaps types between renders.
        let is_over = cursor.is_over(hit);
        if is_over != internal.is_hovered {
            internal.is_hovered = is_over;
            if is_over {
                if let Some(msg) = &self.on_hover_enter {
                    shell.publish(msg.clone());
                }
            } else if let Some(msg) = &self.on_hover_exit {
                shell.publish(msg.clone());
            }
        }

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
        tree: &Tree,
        layout: Layout<'_>,
        cursor: adv_mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        let content_layout = layout.children().next().unwrap();

        let content_interaction = self.content.as_widget().mouse_interaction(
            &tree.children[0],
            content_layout,
            cursor,
            viewport,
            renderer,
        );
        if content_interaction != mouse::Interaction::default() {
            return content_interaction;
        }

        let hit = extended(bounds, viewport);
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
        theme_: &Theme,
        _defaults: &adv_renderer::Style,
        layout: Layout<'_>,
        cursor: adv_mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let content_layout = layout.children().next().unwrap();
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

        let style = (self.style)(theme_, status);

        use iced::advanced::renderer::Renderer as _;
        if style.background.is_some() || style.border.width > 0.0 {
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
            theme_,
            &adv_renderer::Style {
                text_color: style.text_color,
            },
            content_layout,
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
