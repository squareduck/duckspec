# Edge auto-scroll on drag-selection — Design

A shared pure velocity kernel feeds two deliberately different drive mechanisms — a
self-driven redraw loop for the `text_edit` widget and an app-subscription tick for the
`terminal` canvas — plus a chat-fold drain for editors that fit their content but are
clipped by an outer scrollable.

## Approach

The behavior is uniform from the user's side (drag past an edge → the view keeps
scrolling, faster the further out, until the pointer returns inside) but the host
integration differs by surface. One pure kernel computes velocity; each surface owns its
own loop and converts that velocity to its native scroll unit.

```
                     ┌──────────────────────────────┐
                     │  widget::autoscroll           │
                     │  edge_velocity(y, top, bottom)│  pure · unit-tested
                     │   → signed px/frame, ramped,  │  single tuning point
                     │     clamped to MAX            │
                     └──────────────┬───────────────┘
            ┌───────────────────────┼────────────────────────┐
            ▼                       ▼                         ▼
   ┌─────────────────┐   ┌────────────────────┐   ┌────────────────────┐
   │  TEXT EDITOR    │   │     TERMINAL        │   │     CHAT FOLD      │
   │  (Widget)       │   │     (Canvas)        │   │  (editor clipped   │
   │                 │   │                     │   │   by outer scroll) │
   │ self-driven via │   │ app-subscription    │   │ AutoScroll action  │
   │ RedrawRequested │   │ tick @ 16ms gated   │   │ → pending_chat_    │
   │ + request_redraw│   │ on past-edge        │   │   autoscroll       │
   │                 │   │                     │   │ → main drains to   │
   │ → px self-scroll│   │ → line-count scroll │   │   scroll_to(outer) │
   └─────────────────┘   └────────────────────┘   └────────────────────┘
```

Why two drivers rather than one: the editor is a full `Widget` whose drag/scroll state
lives in widget-internal tree state the app can't reach between frames, so it self-drives
off `window::Event::RedrawRequested`. The terminal is a `Canvas` whose scroll state lives
in app-reachable `ix.terminals`, so an app-level subscription tick steps it. Forcing both
into one model would couple widget internals to the app to remove duplication that isn't
accidental — it tracks a real platform boundary. See Decisions.

## Velocity kernel — `widget::autoscroll`

The one shared, pure, independently-testable piece. Maps a pointer position against a
viewport span to a signed per-frame velocity: `0` inside the viewport, ramping linearly
with overshoot distance, clamped to `MAX`. Positive scrolls content up (pointer past the
bottom); negative scrolls down (past the top). Both consumers call this and convert the
result to their own unit.

```rust
/// Per-frame velocity at the edge before ramp (logical px). Low → barely
/// crossing creeps.
const BASE: f32 = 1.0;
/// Extra per-frame velocity per logical px past the edge. Gentle ramp.
const RAMP: f32 = 0.12;
/// Upper bound so a fling to the far side stays controllable (~1200 px/s @60fps).
const MAX: f32 = 20.0;

/// Signed logical px/frame for a pointer at `pointer_y` against `[top, bottom]`.
/// `0.0` inside the span; positive = past bottom; negative = past top.
pub fn edge_velocity(pointer_y: f32, top: f32, bottom: f32) -> f32 { todo!() }

#[cfg(test)]
mod tests { /* inside-viewport=0, past-bottom>0, past-top<0, ramps, clamps, symmetric */ }
```

Constants carry over from the proven implementation unchanged (out of scope to re-tune).
This module has zero coupling to anything else in `duckboard`.

## Text editor drive — `text_edit::{render, state}`

The editor self-scrolls in pixels. A drag `CursorMoved` and every `RedrawRequested` frame
both route through one helper, `drag_frame`, which extends the selection to the
(viewport-clamped) pointer line and, when the pointer is past an edge with room to move,
emits a scroll and re-requests a redraw so the loop keeps running with the mouse held
still.

Two new `InternalState` fields gate the loop against iced's redraw re-dispatch:

```rust
struct InternalState {
    focused: bool,
    dragging: bool,
    cell_width: f32,
    gutter_width: f32,
    link_hover: Option<LinkHover>,
    /// Last frame instant we stepped on. iced re-dispatches the *same*
    /// RedrawRequested(Instant) several times per real frame; step once per
    /// distinct instant or we scroll multiple steps/frame and trip iced's
    /// layout-invalidation guard.
    last_autoscroll_frame: Option<std::time::Instant>,
    /// Whether the drag is currently auto-scrolling. We must re-request a
    /// redraw on *every* dispatch while true — not only the one that steps —
    /// or the loop stalls the instant the mouse stops.
    autoscrolling: bool,
}
```

`drag_frame` detects the edge against the widget's *actually visible* rectangle — the
intersection of layout `bounds` with the clip `viewport` — so it works both for a
self-scrolling file editor (viewport ≈ whole window) and a chat message nested in a
scrollable (viewport = the on-screen slice). It then routes to one of two scroll channels
depending on where the hidden content lives:

```rust
fn drag_frame(
    &self,
    pos: Point,
    bounds: Rectangle,
    viewport: Rectangle,
    internal: &InternalState,
    wrap: Option<&WrapLayout>,
    shell: &mut Shell<'_, M>,
) -> bool {
    let top = bounds.y.max(viewport.y);
    let bottom = (bounds.y + bounds.height).min(viewport.y + viewport.height);
    let velocity = autoscroll::edge_velocity(pos.y, top, bottom);

    // Selection always tracks the edge line: clamp the drag target into the
    // visible span so a big overshoot doesn't snap selection to the doc end.
    let drag_pos = pixel_to_pos_wrapped(Point::new(pos.x, pos.y.clamp(top, bottom)), /* … */);
    shell.publish((self.on_action)(EditorAction::Drag(drag_pos)));

    if velocity == 0.0 || self.static_viewport { return false; }

    // Only scroll if there is hidden content in that direction:
    //  owns_scroll  → editor's own overflow → EditorAction::Scroll { dy: velocity, … }
    //  clipped by viewport (fits content, sits in outer scroll) → EditorAction::AutoScroll { dy }
    // … then shell.request_redraw(); to keep the loop alive.
    todo!()
}
```

Hooked into `update` at two events:

```rust
Event::Mouse(mouse::Event::CursorMoved { .. }) => {
    // cursor.land().position() so a pointer dragged past an enclosing
    // scrollable's fold (arrives Levitating, position()==None) still yields a
    // point — exactly when a nested chat message needs to auto-scroll.
    if internal.dragging && internal.focused && let Some(pos) = cursor.land().position() {
        self.drag_frame(pos, bounds, *viewport, internal, wrap.as_ref(), shell);
    } /* else hover detection */
}
Event::Window(window::Event::RedrawRequested(now)) => {
    if internal.dragging && internal.focused && let Some(pos) = cursor.land().position() {
        if internal.last_autoscroll_frame != Some(*now) {
            internal.last_autoscroll_frame = Some(*now);
            internal.autoscrolling = self.drag_frame(pos, bounds, *viewport, internal, wrap.as_ref(), shell);
        }
        if internal.autoscrolling { shell.request_redraw(); }
    } else {
        internal.autoscrolling = false;
    }
}
```

New editor action, a no-op in `apply_action` because the only editors that emit it have
nothing of their own to move — the host scrolls the outer container:

```rust
pub enum EditorAction {
    // … Drag, DragEnd, Scroll { … }, SaveRequested …
    /// Drag ran past an edge of an editor that fits its content inside an
    /// *outer* scrollable (a chat message body). `dy` is logical px to move
    /// that outer container; positive scrolls toward the end.
    AutoScroll { dy: f32 },
}
```

## Terminal drive — `terminal`

The terminal is a canvas; its scroll state lives in `ix.terminals`, so the app ticks it.
On each drag `CursorMoved` the canvas records the live pointer; a 60fps subscription
(gated so it only fires while a drag holds the pointer past an edge) steps every
autoscrolling terminal, converting px velocity to a line count.

```rust
/// Last drag pointer in canvas-local px + the viewport height it was measured
/// against, so the tick can recompute velocity without a fresh mouse event.
#[derive(Clone, Copy)]
struct DragPointer { x: f32, y: f32, viewport_height: f32 }

pub struct TerminalState {
    // … pending_scroll, pending_selection …
    /// Live drag pointer for continuous edge auto-scroll; None when no drag.
    drag_pointer: Cell<Option<DragPointer>>,
}

impl TerminalState {
    /// Record (or clear with None) the live drag pointer. Called from canvas
    /// update() on CursorMoved while dragging, and with None on ButtonReleased.
    pub fn set_drag_pointer(&self, pointer: Option<(f32, f32, f32)>) { todo!() }

    /// Drag active *and* pointer past a vertical edge — gates the subscription.
    pub fn is_drag_autoscrolling(&self) -> bool {
        self.drag_pointer.get().is_some_and(|dp|
            autoscroll::edge_velocity(dp.y, 0.0, dp.viewport_height) != 0.0)
    }

    /// Advance one frame: scroll the display toward the edge and re-extend the
    /// selection. `edge_velocity` is positive past the *bottom*; the terminal
    /// reveals lines below via a *negative* display delta, so invert; round
    /// away from zero so each frame moves ≥1 line.
    pub fn drag_autoscroll_step(&mut self) -> bool { todo!() }
}
```

## App tick + chat-fold drain — `main`, `area::interaction`

The terminal subscription and message:

```rust
// in Message
TerminalAutoscrollTick,

// in update()
Message::TerminalAutoscrollTick => {
    for ix in state.interactions.values_mut() {
        for tt in &mut ix.terminals {
            if tt.state.is_drag_autoscrolling() { tt.state.drag_autoscroll_step(); }
        }
    }
}

// in subscription() — only while something is past an edge, ~60fps
if any_terminal_autoscrolling(state) {
    subs.push(iced::time::every(Duration::from_millis(16)).map(|_| Message::TerminalAutoscrollTick));
}
fn any_terminal_autoscrolling(state: &State) -> bool { /* any ix.terminals … is_drag_autoscrolling */ }
```

A chat message editor that emits `EditorAction::AutoScroll` can't move itself, so the
session accumulates the request and `main` drains it after dispatch into an absolute
scroll on the chat scrollable:

```rust
// SessionState (area::interaction)
/// Accumulated edge auto-scroll delta (logical px) from a chat message whose
/// drag ran past the chat fold. Drained by main into a scroll_to. None when idle.
pub pending_chat_autoscroll: Option<f32>,

// agent_chat::Msg::ChatAction handling
if let text_edit::EditorAction::AutoScroll { dy } = action {
    ax.pending_chat_autoscroll = Some(ax.pending_chat_autoscroll.unwrap_or(0.0) + dy);
    ax.stick_to_bottom = false; // a later streaming snap must not fight this scroll
    return;
}
```

The drain converts the accumulated delta to an absolute offset (advancing
`last_chat_offset_y` so the scroll-preservation replay stays consistent) and is issued
*instead of* the snapshot replay — replaying the pre-update offset would immediately undo
the deliberate scroll:

```rust
fn has_pending_chat_autoscroll(state: &State) -> bool { /* any session … is_some */ }

fn take_pending_chat_autoscroll(state: &mut State) -> Task<Message> {
    // for each session: y = (last_chat_offset_y + dy).max(0); last_chat_offset_y = y;
    // scroll_to(CHAT_SCROLLABLE_ID, AbsoluteOffset { x: 0, y })
    todo!()
}

// at the chat-scroll reconciliation point in update():
if has_pending_chat_autoscroll(state) {
    return Task::batch([task, take_pending_chat_autoscroll(state)]); // skip replay
}
```

## Decisions

- **Two drive mechanisms, not one** — editor self-drives off `RedrawRequested`; terminal
  is stepped by an app subscription. Alternatives: (a) unify on an app tick — rejected,
  the editor's drag/scroll state lives in widget-internal tree state the app can't reach,
  so it would require hoisting widget internals into app state and coupling the two; (b)
  unify on a self-driven canvas redraw loop — rejected, the canvas doesn't get window
  redraw events the same way. The split tracks a real iced widget-vs-canvas boundary, so
  the duplication is not accidental and is left in place.

- **Shared pure kernel** — factor `edge_velocity` into its own module rather than inlining
  the ramp in each consumer. Gives one unit-tested definition and one place to tune
  `BASE`/`RAMP`/`MAX`. Mirrors the house pattern where `parse/elements` is the single
  shared layer the other parsers consume.

- **Viewport-clamped drag target** — clamp the selection's drag point into the visible
  span before extending. Alternative: use the raw pointer position — rejected, a large
  overshoot would snap the selection straight to the document end instead of tracking the
  edge line as content scrolls in.

- **Separate `AutoScroll` action for the chat fold** — a clipped-but-fitting editor emits
  `AutoScroll { dy }` (host moves the outer scrollable) rather than `Scroll`
  (self-scroll). Alternative: make the editor self-scroll — impossible, it has no overflow
  of its own; the outer scrollable owns the hidden content.

- **Bypass scroll-preservation replay on pending chat autoscroll** — when a chat drag
  scroll is pending, issue it and skip the snapshot replay. Alternative: let the replay
  run — rejected, it would restore the pre-update offset and cancel the scroll every
  frame.

## Risks

- **iced re-dispatches the same `RedrawRequested(Instant)` several times per real frame**
  → step at most once per distinct instant via `last_autoscroll_frame`; multiple
  steps/frame would trip iced's consecutive-layout-invalidation guard.

- **Self-driven loop stalls the moment the mouse stops moving** → re-request a redraw on
  *every* dispatch while `autoscrolling`, not only on the stepping one, so iced's redraw
  loop carries `NextFrame` and keeps spinning.

- **`stick_to_bottom` (streaming snap) fights a chat autoscroll** → drop `stick_to_bottom`
  the moment an `AutoScroll` is accumulated.

## Open questions

None. All hook points (`CHAT_SCROLLABLE_ID`, `last_chat_offset_y`, the scroll-preservation
snapshot/replay, `ix.terminals` / `ix.sessions`) exist on the current board and matched
the reference diff line-for-line; remaining work is mechanical porting.
