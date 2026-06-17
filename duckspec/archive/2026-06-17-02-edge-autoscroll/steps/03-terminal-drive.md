# Terminal drive

Track the live drag pointer on the terminal canvas and expose the per-frame step the app
tick will call, converting pixel velocity to a line-count scroll.

## Prerequisites

- [ ] @step velocity-kernel

## Tasks

- [x] 1. In `widget/terminal.rs`, add the
         `#[derive(Clone, Copy)] struct
         DragPointer { x: f32, y: f32, viewport_height: f32 }`
         and a `drag_pointer:
         Cell<Option<DragPointer>>` field on `TerminalState`,
         initialized to `None` in its constructor. Add `use crate::widget::autoscroll;`.

- [x] 2. Add `pub fn set_drag_pointer(&self, pointer: Option<(f32, f32, f32)>)` that
         records or clears the live pointer.

- [x] 3. Add `pub fn is_drag_autoscrolling(&self) -> bool` returning true when a drag
         pointer is set and `edge_velocity(dp.y, 0.0, dp.viewport_height) != 0.0` — this
         gates the app subscription.

- [x] 4. Add `pub fn drag_autoscroll_step(&mut self) -> bool`: recompute velocity, convert
         to a line count (`-(velocity / cell_height())`, rounded away from zero so each
         frame moves ≥1 line — note the sign inversion, terminal reveals lines below via a
         negative display delta), then `request_scroll`,
         `queue_selection_update(dp.x, dp.y)`, and `apply_scroll`. Return whether it
         scrolled.

- [x] 5. In the canvas `update` drag branch (`CursorMoved` while dragging), call
         `set_drag_pointer(Some((pos.x, pos.y, bounds.height)))` so the tick can keep
         scrolling while the mouse is held still; remove the old inline scroll-past-edge
         block now superseded by the tick.

- [x] 6. On `ButtonReleased` (drag end), call `set_drag_pointer(None)` to stop the
         continuous scroll.
