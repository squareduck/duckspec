# Text editor drive

Wire the text editor to self-scroll in pixels while a drag holds the pointer past a
vertical edge, driving the loop off `RedrawRequested` so it continues when the mouse is
still.

## Prerequisites

- [ ] @step velocity-kernel

## Tasks

- [x] 1. In `widget/text_edit/state.rs`, add the `AutoScroll { dy: f32 }` variant to
         `EditorAction` (with the design's doc comment) and a no-op arm in `apply_action`
         — editors that emit it have no overflow of their own to move.

- [x] 2. In `widget/text_edit/render.rs`, add the
         `last_autoscroll_frame:
         Option<std::time::Instant>` and `autoscrolling: bool`
         fields to `InternalState` (with doc comments) and initialize them in its
         constructor.

- [x] 3. Add the `drag_frame(&self, pos, bounds, viewport, internal, wrap,
         shell) -> bool`
         helper: compute the visible span (`bounds ∩ viewport`), call
         `autoscroll::edge_velocity`, publish a viewport-clamped `EditorAction::Drag`,
         then — when past an edge with room to move — route to `EditorAction::Scroll`
         (owns overflow) or `EditorAction::AutoScroll` (clipped by the outer scrollable)
         and `shell.request_redraw()`. Return whether it scrolled.

- [x] 4. Replace the drag branch of the `CursorMoved` handler in `update` to call
         `drag_frame` using `cursor.land().position()` (so a pointer past an enclosing
         scrollable's fold still yields a position).

- [x] 5. Add a `window::Event::RedrawRequested(now)` arm in `update` that steps
         `drag_frame` at most once per distinct instant (guarded by
         `last_autoscroll_frame`), stores the result in `autoscrolling`, and re-requests a
         redraw on every dispatch while `autoscrolling` is true; clears `autoscrolling`
         when no drag is active.

- [x] 6. Add the `use crate::widget::autoscroll;` import to `render.rs`.
