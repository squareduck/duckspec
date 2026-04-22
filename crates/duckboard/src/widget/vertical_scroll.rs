//! Vertical scroll wrapper without iced scrollable's wheel-event transaction
//! lock.
//!
//! `iced::widget::scrollable` claims the scroll wheel for ~1.5 s after it
//! consumes a scroll event, blocking all wheel events from reaching
//! descendants. That lock is what stops nested horizontal-pan widgets from
//! receiving a horizontal swipe right after a vertical one. This widget
//! always forwards events to its child first, then handles the vertical
//! component itself — so an inner horizontal pan can act on the same wheel
//! event in a different axis without contention.
//!
//! Renders the same 4 px thin scrollbar overlay the rest of the chrome uses.
//!
//! Controlled component: the parent owns the offset and updates it via the
//! `on_scroll` callback. This keeps scroll position stable across data
//! refreshes and structural rebuilds (where iced's per-tree widget state
//! would otherwise be discarded).

use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::advanced::{layout, mouse as adv_mouse, renderer as adv_renderer};
use iced::{Border, Element, Event, Length, Rectangle, Size, Theme, Vector, mouse};

use crate::theme;

const SCROLLBAR_WIDTH: f32 = 4.0;
const SCROLLBAR_RADIUS: f32 = 2.0;
const SCROLLBAR_MIN_SCROLLER: f32 = 20.0;

pub struct VerticalScroll<'a, M> {
    content: Element<'a, M>,
    offset_y: f32,
    on_scroll: Box<dyn Fn(f32) -> M + 'a>,
}

impl<'a, M> VerticalScroll<'a, M> {
    pub fn new(
        offset_y: f32,
        on_scroll: impl Fn(f32) -> M + 'a,
        content: impl Into<Element<'a, M>>,
    ) -> Self {
        Self {
            content: content.into(),
            offset_y: offset_y.max(0.0),
            on_scroll: Box::new(on_scroll),
        }
    }

    /// Effective offset for layout/draw — never below 0, never past
    /// `max_offset`. Lets us survive a momentary content shrink without
    /// snapping the externally-stored offset to 0 on every reload.
    fn effective_offset(&self, max_offset: f32) -> f32 {
        self.offset_y.clamp(0.0, max_offset)
    }
}

impl<'a, M: Clone> Widget<M, Theme, iced::Renderer> for VerticalScroll<'a, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn state(&self) -> tree::State {
        tree::State::None
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let viewport_w = limits.max().width;
        let viewport_h = limits.max().height;

        // Mirror iced::scrollable's child limits: bound width to the
        // viewport, allow unbounded height, and mark height as compressed so
        // Fill-height descendants resolve to their intrinsic heights instead
        // of expanding to infinity.
        let content_limits = layout::Limits::with_compression(
            limits.min(),
            Size::new(viewport_w, f32::INFINITY),
            Size::new(false, true),
        );
        let content_node =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &content_limits);

        layout::Node::with_children(Size::new(viewport_w, viewport_h), vec![content_node])
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
        let bounds = layout.bounds();
        let content_layout = layout.children().next().unwrap();
        let content_bounds = content_layout.bounds();
        let max_offset = (content_bounds.height - bounds.height).max(0.0);
        let effective = self.effective_offset(max_offset);

        let cursor_for_child = cursor + Vector::new(0.0, effective);

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            content_layout,
            cursor_for_child,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event
            && cursor.is_over(bounds)
            && max_offset > 0.0
        {
            let dy = match delta {
                mouse::ScrollDelta::Lines { y, .. } => -*y * 60.0,
                mouse::ScrollDelta::Pixels { y, .. } => -*y,
            };
            if dy != 0.0 {
                let new_offset = (effective + dy).clamp(0.0, max_offset);
                if new_offset != effective {
                    shell.publish((self.on_scroll)(new_offset));
                    shell.request_redraw();
                }
                // Intentionally not capturing — nested horizontal_pan will
                // have already consumed dx during the forward above; nothing
                // else above us cares about this wheel event.
            }
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
        let content_bounds = layout.children().next().unwrap().bounds();
        let max_offset = (content_bounds.height - bounds.height).max(0.0);
        let effective = self.effective_offset(max_offset);
        let cursor_for_child = if cursor.is_over(bounds) {
            cursor + Vector::new(0.0, effective)
        } else {
            cursor
        };
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap(),
            cursor_for_child,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        defaults: &adv_renderer::Style,
        layout: Layout<'_>,
        cursor: adv_mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let Some(visible) = bounds.intersection(viewport) else {
            return;
        };
        let content_layout = layout.children().next().unwrap();
        let content_bounds = content_layout.bounds();
        let max_offset = (content_bounds.height - bounds.height).max(0.0);
        let effective = self.effective_offset(max_offset);
        let translation = Vector::new(0.0, -effective);

        let cursor_for_child = if cursor.is_over(bounds) {
            cursor - translation
        } else {
            adv_mouse::Cursor::Unavailable
        };

        use iced::advanced::renderer::Renderer as _;
        renderer.with_layer(visible, |renderer| {
            renderer.with_translation(translation, |renderer| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    defaults,
                    content_layout,
                    cursor_for_child,
                    &Rectangle {
                        x: visible.x - translation.x,
                        y: visible.y - translation.y,
                        ..visible
                    },
                );
            });
        });

        // Thin scrollbar overlay. Drawn in its own layer so it paints on
        // top of the content layer above — without `with_layer`, sibling
        // section headers' opaque backgrounds occlude the scroller.
        if content_bounds.height > bounds.height && bounds.height > 0.0 {
            let track_h = bounds.height;
            let ratio = (track_h / content_bounds.height).clamp(0.0, 1.0);
            let scroller_h = (track_h * ratio).max(SCROLLBAR_MIN_SCROLLER).min(track_h);
            let max_scroll_y = content_bounds.height - track_h;
            let t = if max_scroll_y > 0.0 {
                (effective / max_scroll_y).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let scroller_y = bounds.y + (track_h - scroller_h) * t;
            let scroller_bounds = Rectangle {
                x: bounds.x + bounds.width - SCROLLBAR_WIDTH,
                y: scroller_y,
                width: SCROLLBAR_WIDTH,
                height: scroller_h,
            };
            renderer.with_layer(visible, |renderer| {
                renderer.fill_quad(
                    adv_renderer::Quad {
                        bounds: scroller_bounds,
                        border: Border {
                            radius: SCROLLBAR_RADIUS.into(),
                            ..Border::default()
                        },
                        ..adv_renderer::Quad::default()
                    },
                    theme::text_muted(),
                );
            });
        }
    }
}

impl<'a, M: Clone + 'a> From<VerticalScroll<'a, M>> for Element<'a, M> {
    fn from(s: VerticalScroll<'a, M>) -> Self {
        Element::new(s)
    }
}

pub fn view<'a, M: Clone + 'a>(
    offset_y: f32,
    on_scroll: impl Fn(f32) -> M + 'a,
    content: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    VerticalScroll::new(offset_y, on_scroll, content).into()
}
