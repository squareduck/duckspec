# Edge auto-scroll on drag-selection

When a user drags a text selection past the top or bottom edge of a viewport, the view
scrolls on its own — ramping with overshoot — so the selection keeps extending beyond
what's visible. Applies to the text editor, the embedded terminal, and chat messages
clipped by the chat fold.

## Motivation

A selection can't currently extend past what's on screen — the user must release the drag,
scroll, and re-drag. This is proven behavior salvaged from the discarded ducknest stack:
it is the last open item on the post-ducknest salvage list, the rest of which already
landed in `spec-graph-integrity`. Reintroducing it closes out that salvage work.

## Scope

This is a pure GUI port with no capability specs. The only unit-tested piece is the
velocity kernel; everything else is interaction plumbing that capability specs would not
meaningfully pin down.

```
widget/autoscroll.rs       (new) — pure edge_velocity() ramp/clamp kernel + unit tests
widget/terminal.rs               — drag-pointer tracking + app-tick step (line scroll)
widget/text_edit/render.rs       — drag_frame self-driven redraw loop
widget/text_edit/state.rs        — AutoScroll editor action (pixel scroll)
area/interaction.rs              — pending_chat_autoscroll channel
main.rs                          — tick message + subscription + chat-fold drain
```

### Components

- `widget/autoscroll.rs` *(new)* — pure `edge_velocity()` ramp/clamp kernel and its unit
  tests. Maps pointer position plus viewport bounds to a signed per-frame scroll velocity
  that ramps linearly with overshoot and clamps to a max. The only unit-tested piece.

- `widget/terminal.rs` — live drag-pointer tracking plus an app-tick step that scrolls the
  terminal by a line count.

- `widget/text_edit/render.rs`, `widget/text_edit/state.rs` — the `drag_frame` self-driven
  redraw loop (pixel scroll) and a new `AutoScroll` editor action for editors that can't
  self-scroll.

- `area/interaction.rs`, `main.rs` — the `pending_chat_autoscroll` channel, the
  auto-scroll tick message and subscription, and the chat-fold drain that moves the outer
  scrollable.

### Out of scope

- Horizontal auto-scroll — vertical only.

- Tuning the `BASE` / `RAMP` / `MAX` velocity constants — carry the proven values over
  as-is.

- Auto-scroll on any other widget (lists, trees).

- Anything from the discarded ducknest daemon / Telegram stack.

## Impact

Additive to `duckboard` only — no `duckpond` library changes, no API or breaking changes,
and no new dependencies. All hook points this wires into (`ix.terminals`, `ix.sessions`,
`last_chat_offset_y`, `stick_to_bottom`, the scroll-preservation replay) already exist on
the current board.
