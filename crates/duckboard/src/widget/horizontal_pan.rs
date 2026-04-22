//! Horizontal pan wrapper for a single child.
//!
//! Lays out the child at its intrinsic width (uncapped) and clips it to the
//! widget's allocated width, exposing horizontal trackpad/wheel scrolling
//! within the visible region. Vertical wheel events pass through to outer
//! scrollables.
//!
//! Used to let long list rows (deeply nested file paths, capability ids)
//! pan within the fixed-width list column without wrapping or truncating.

use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::advanced::{layout, mouse as adv_mouse, renderer as adv_renderer};
use iced::{Element, Event, Length, Rectangle, Size, Theme, Vector, mouse};

#[derive(Debug, Default)]
struct InternalState {
    offset_x: f32,
}

pub struct HorizontalPan<'a, M> {
    content: Element<'a, M>,
}

impl<'a, M> HorizontalPan<'a, M> {
    pub fn new(content: impl Into<Element<'a, M>>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

impl<'a, M: Clone> Widget<M, Theme, iced::Renderer> for HorizontalPan<'a, M> {
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
        Size {
            width: Length::Fill,
            height: Length::Shrink,
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

        // Pass 1 — measure the child's intrinsic content width by laying it
        // out under width-compression with an unbounded width budget. Fluid
        // (Length::Fill) descendants collapse to their intrinsic widths in
        // this mode, so the resulting size reports the natural width of the
        // widest row.
        let measure_limits = layout::Limits::with_compression(
            Size::ZERO,
            Size::new(f32::INFINITY, viewport_h),
            Size::new(true, false),
        );
        let measure_node =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &measure_limits);
        let intrinsic_w = measure_node.size().width;

        // Pass 2 — re-lay out with a fixed width = max(intrinsic, viewport).
        // This makes Length::Fill descendants stretch to the natural max-row
        // width, so selection/hover backgrounds always span the entire row
        // (and remain visible across the whole viewport even when the row
        // text is shorter than the viewport).
        let render_w = intrinsic_w.max(viewport_w);
        let render_limits =
            layout::Limits::new(Size::new(render_w, 0.0), Size::new(render_w, viewport_h));
        let content_node =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &render_limits);
        let content_h = content_node.size().height;

        layout::Node::with_children(Size::new(viewport_w, content_h), vec![content_node])
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
        let internal = tree.state.downcast_mut::<InternalState>();

        let max_offset = (content_bounds.width - bounds.width).max(0.0);
        internal.offset_x = internal.offset_x.clamp(0.0, max_offset);

        // Translate the cursor into child coordinates so widgets inside the
        // pan see the correct hit-testing position.
        let cursor_for_child = cursor + Vector::new(internal.offset_x, 0.0);

        // Forward a viewport that represents the visible window in our
        // content's coordinate space. PanRow uses this to extend its hover
        // background and click hit-area across the full visible width.
        let viewport_for_child = Rectangle {
            x: bounds.x + internal.offset_x,
            y: viewport.y,
            width: bounds.width,
            height: viewport.height,
        };

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            content_layout,
            cursor_for_child,
            renderer,
            clipboard,
            shell,
            &viewport_for_child,
        );

        if shell.is_event_captured() {
            return;
        }

        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event
            && cursor.is_over(bounds)
            && max_offset > 0.0
        {
            let dx = match delta {
                mouse::ScrollDelta::Lines { x, .. } => -*x * 60.0,
                mouse::ScrollDelta::Pixels { x, .. } => -*x,
            };
            if dx != 0.0 {
                let new_offset = (internal.offset_x + dx).clamp(0.0, max_offset);
                if new_offset != internal.offset_x {
                    internal.offset_x = new_offset;
                    shell.request_redraw();
                }
                // Intentionally not capturing — vertical wheel components
                // still need to reach the outer scrollable.
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
        let internal = tree.state.downcast_ref::<InternalState>();
        let cursor_for_child = if cursor.is_over(bounds) {
            cursor + Vector::new(internal.offset_x, 0.0)
        } else {
            cursor
        };
        let viewport_for_child = Rectangle {
            x: bounds.x + internal.offset_x,
            y: viewport.y,
            width: bounds.width,
            height: viewport.height,
        };
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap(),
            cursor_for_child,
            &viewport_for_child,
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
        let internal = tree.state.downcast_ref::<InternalState>();
        let translation = Vector::new(-internal.offset_x, 0.0);

        let cursor_for_child = if cursor.is_over(bounds) {
            cursor - translation
        } else {
            adv_mouse::Cursor::Unavailable
        };

        use iced::advanced::Renderer as _;
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
    }
}

impl<'a, M: Clone + 'a> From<HorizontalPan<'a, M>> for Element<'a, M> {
    fn from(pan: HorizontalPan<'a, M>) -> Self {
        Element::new(pan)
    }
}

/// Wrap `content` in a horizontal pan region. When `content` is wider than
/// the available width, trackpad/wheel horizontal gestures pan it within the
/// viewport. Vertical wheel events are not consumed.
pub fn view<'a, M: Clone + 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    HorizontalPan::new(content).into()
}
