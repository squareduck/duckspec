# Cmd-left realign to current answer - Design

Previous-answer jumps re-align to the current Answer top when the viewport is below it;
only then step to the prior Answer. Next-answer path unchanged. Logic stays pure in
`agent_chat`; the layout Operation is the single caller.

## Approach

Extend target selection inside the existing ⌘← path. No new shortcuts, no `main.rs`
keybind changes beyond whatever already calls `scroll_to_adjacent_answer`.

```
⌘←  →  apply_chat_landmark(PrevAnswer)
              │
              ▼
     scroll_to_adjacent_answer(go_prev=true)
              │
              ▼
     ScrollToAdjacentAnswer::finish
              │
              ├─ measure tops + scrollable origin
              ├─ current = current_answer_for_reply_jumps(...)  (unchanged)
              │
              ├─ go_prev?
              │     yes → target = prev with re-align-first
              │     no  → target = next_answer_idx (unchanged)
              │
              └─ scroll_to(top of target)
```

Re-align condition (same for stick-to-bottom and mid-message):

```
offset_y > current_top + VIEWPORT_TOP_EPS  →  target = current
else                                        →  target = prev_answer_idx(...)
```

Use the scrollable’s **layout `translation.y`** for that comparison (and for non-stick
current resolution) so stick-to-bottom does not depend on a possibly-stale
`last_chat_offset_y` of `0.0`. Keep the `stick_to_bottom` flag for “current = last Answer”
only.

## Reply-jump target helper

New pure function next to the existing anchor helpers in
`crates/duckboard/src/widget/agent_chat.rs`:

```rust
/// Prev jump: re-align to `current` when viewport is below its top; else prior Answer.
/// Next jump: adjacent next only (no re-align-first).
pub fn target_answer_for_reply_jump(
    anchors: &[usize],
    answer_tops: &[(usize, f32)],
    current: Option<usize>,
    go_prev: bool,
    offset_y: f32,
) -> Option<usize>
```

Sketch of the prev branch:

```rust
if go_prev {
    if let Some(cur) = current {
        if let Some(&(_, top)) = answer_tops.iter().find(|(i, _)| *i == cur) {
            if offset_y > top + VIEWPORT_TOP_EPS {
                return Some(cur);
            }
        }
    }
    return prev_answer_idx(anchors, current);
}
next_answer_idx(anchors, current)
```

Reuse private `VIEWPORT_TOP_EPS` (1.0). Unit tests cover: below top → current; at top
(±eps) → previous; first Answer at top → `None`; next → never re-aligns.

## ScrollToAdjacentAnswer operation

In `finish`, replace bare `prev_answer_idx` / `next_answer_idx` with
`target_answer_for_reply_jump`. Capture `translation.y` in `scrollable(...)` so offset is
measured, not only passed from interaction state.

```rust
// fields (add)
measured_offset_y: Option<f32>,

// scrollable callback
self.scrollable_y = Some(bounds.y);
self.measured_offset_y = Some(translation.y);

// finish
let offset_y = self.measured_offset_y.unwrap_or(self.offset_y);
let current = current_answer_for_reply_jumps(
    &anchors, &tops, offset_y, self.stick_to_bottom,
);
let target = target_answer_for_reply_jump(
    &anchors, &tops, current, self.go_prev, offset_y,
);
// scroll to measured top of target (existing)
```

`scroll_to_adjacent_answer` signature can keep the existing `offset_y` / `stick_to_bottom`
parameters as fallbacks when layout has not reported translation yet.

`apply_chat_landmark` in `main.rs` stays: unstick, materialize dirty chat UI, call
`scroll_to_adjacent_answer` — no re-align logic at the app layer.

## Capability docs / specs

Delta on `duckspec/caps/chat/answer-landmarks/` only:

```
| Artifact | Change |
| --- | --- |
| `doc.md` | Document re-align-first for previous; next unchanged |
| `spec.md` | New or extended requirement + scenarios for prev re-align / at-top steps prev / next no re-align |
```

No new capability path.

## Impact

- Code: `crates/duckboard/src/widget/agent_chat.rs` (helpers + Operation + unit tests)
- Specs: `duckspec/caps/chat/answer-landmarks/{spec,doc}.md`
- No API/crate boundary change; no migrations; ⌘→, history ends, anchors, band unchanged

## Decisions

- **Measured scroll offset in the Operation** — prefer `translation.y` for re-align and
  non-stick current. Alternative: only `last_chat_offset_y` from interaction state
  (rejected: stick + missing/stale offset can skip re-align and recreate the bug).

- **Pure target helper** — keep policy unit-testable without iced layout. Alternative:
  only branch inside `finish` (rejected: harder to backlink scenarios).

- **No stick-only special case** — stick is only for current-Answer resolution; re-align
  is always “offset below top.” Alternative: `if stick { re-align }` always (rejected:
  proposal asked for one rule; short fully-visible last answer still re-aligns once via
  offset comparison when stuck at end).

- **⌘→ untouched** — adjacent next only.

## Risks

- **Content shorter than viewport** (stick, offset ≈ 0, last top ≈ 0) → first ⌘← may step
  previous immediately; acceptable (nothing to re-align).

- **Missing block id / unmeasured top** → no re-align branch, fall through to prev (same
  as today’s missing-target no-op paths).
