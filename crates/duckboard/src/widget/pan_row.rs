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
    Background, Color, Element, Event, Length, Padding, Rectangle, Size, Theme, Vector, mouse,
    touch,
};

use crate::theme;

type StyleFn = fn(&Theme, button::Status) -> button::Style;

/// Gap between a sticky-trailing element and the right edge of the visible
/// viewport. Also used as horizontal inset for the sticky's backdrop quad.
const STICKY_GAP: f32 = theme::SPACING_SM;

#[derive(Debug, Default)]
struct InternalState {
    is_pressed: bool,
}

pub struct PanRow<'a, M> {
    content: Element<'a, M>,
    sticky_trailing: Option<Element<'a, M>>,
    on_press: Option<M>,
    style: StyleFn,
    padding: Padding,
}

impl<'a, M: Clone> PanRow<'a, M> {
    pub fn new(content: impl Into<Element<'a, M>>) -> Self {
        Self {
            content: content.into(),
            sticky_trailing: None,
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

    /// An element pinned to the right edge of the visible viewport rather
    /// than the row's natural width. Stays in place as the user pans
    /// horizontally — used for affordances like a tab close button that
    /// must remain reachable regardless of pan offset.
    pub fn sticky_trailing(mut self, el: impl Into<Element<'a, M>>) -> Self {
        self.sticky_trailing = Some(el.into());
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

/// Translation that moves the sticky child from its laid-out position
/// (inside the row at x=0, vertically centered) to its drawn position at
/// the right edge of the visible viewport.
fn sticky_translation(sticky_layout: Layout<'_>, row: Rectangle, viewport: &Rectangle) -> Vector {
    let st = sticky_layout.bounds();
    let target_x = viewport.x + viewport.width - st.width - STICKY_GAP;
    let target_y = row.y + (row.height - st.height) / 2.0;
    Vector::new(target_x - st.x, target_y - st.y)
}

impl<'a, M: Clone> Widget<M, Theme, iced::Renderer> for PanRow<'a, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<InternalState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(InternalState::default())
    }

    fn children(&self) -> Vec<Tree> {
        match &self.sticky_trailing {
            Some(st) => vec![Tree::new(&self.content), Tree::new(st)],
            None => vec![Tree::new(&self.content)],
        }
    }

    fn diff(&self, tree: &mut Tree) {
        match &self.sticky_trailing {
            Some(st) => tree.diff_children(&[&self.content, st]),
            None => tree.diff_children(std::slice::from_ref(&self.content)),
        }
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

        let mut children = vec![content_node];
        if let Some(sticky) = &mut self.sticky_trailing {
            let sticky_node = sticky.as_widget_mut().layout(
                &mut tree.children[1],
                renderer,
                &layout::Limits::new(Size::ZERO, Size::INFINITE),
            );
            let st_h = sticky_node.size().height;
            let y = ((row_size.height - st_h) / 2.0).max(0.0);
            children.push(sticky_node.move_to((0.0, y)));
        }

        layout::Node::with_children(row_size, children)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        let mut children = layout.children();
        let content_layout = children.next().unwrap();
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            content_layout,
            renderer,
            operation,
        );
        if let Some(sticky) = &mut self.sticky_trailing {
            let sticky_layout = children.next().unwrap();
            sticky.as_widget_mut().operate(
                &mut tree.children[1],
                sticky_layout,
                renderer,
                operation,
            );
        }
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
        let mut layout_children = layout.children();
        let content_layout = layout_children.next().unwrap();

        // Sticky goes first so it can capture clicks before the row's
        // viewport-wide hit area swallows them.
        if let Some(sticky) = &mut self.sticky_trailing {
            let sticky_layout = layout_children.next().unwrap();
            let translation = sticky_translation(sticky_layout, bounds, viewport);
            sticky.as_widget_mut().update(
                &mut tree.children[1],
                event,
                sticky_layout,
                cursor - translation,
                renderer,
                clipboard,
                shell,
                viewport,
            );
            if shell.is_event_captured() {
                return;
            }
        }

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
        let mut layout_children = layout.children();
        let content_layout = layout_children.next().unwrap();

        if let Some(sticky) = &self.sticky_trailing {
            let sticky_layout = layout_children.next().unwrap();
            let translation = sticky_translation(sticky_layout, bounds, viewport);
            let drawn = sticky_layout.bounds() + translation;
            if cursor.is_over(drawn) {
                return sticky.as_widget().mouse_interaction(
                    &tree.children[1],
                    sticky_layout,
                    cursor - translation,
                    viewport,
                    renderer,
                );
            }
        }

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
        let mut layout_children = layout.children();
        let content_layout = layout_children.next().unwrap();
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

        if let Some(sticky) = &self.sticky_trailing {
            let sticky_layout = layout_children.next().unwrap();
            let translation = sticky_translation(sticky_layout, bounds, viewport);
            let drawn = sticky_layout.bounds() + translation;

            // Backdrop spans from a small inset before the sticky to the
            // viewport's right edge, so panned text doesn't bleed under
            // the sticky element. Match the row's effective background;
            // fall back to the panel surface tone for transparent rows.
            let backdrop_bg = match style.background {
                Some(bg) => bg,
                None => Background::Color(theme::bg_surface()),
            };
            let backdrop = Rectangle {
                x: drawn.x - STICKY_GAP,
                y: bounds.y,
                width: drawn.width + 2.0 * STICKY_GAP,
                height: bounds.height,
            };
            renderer.fill_quad(
                adv_renderer::Quad {
                    bounds: backdrop,
                    ..adv_renderer::Quad::default()
                },
                backdrop_bg,
            );

            renderer.with_translation(translation, |renderer| {
                sticky.as_widget().draw(
                    &tree.children[1],
                    renderer,
                    theme_,
                    &adv_renderer::Style {
                        text_color: style.text_color,
                    },
                    sticky_layout,
                    cursor - translation,
                    viewport,
                );
            });
        }
    }
}

impl<'a, M: Clone + 'a> From<PanRow<'a, M>> for Element<'a, M> {
    fn from(r: PanRow<'a, M>) -> Self {
        Element::new(r)
    }
}
