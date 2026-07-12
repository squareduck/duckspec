# Equal chat / content split - Design

Uncustomized interaction panels track half of free horizontal space; first grip drag locks
absolute pixel width. No-content fill stays as today.

## Approach

Keep `InteractionState::width` as the laid-out chat width when content is visible. Add a
session-only `width_customized` flag. Drive uncustomized width from window size so resize
and first open stay 50/50 without switching the view to `FillPortion`.

```
window width
    │
    ▼
free = W − sidebar − list − dividers − handle
    │
    ├── uncustomized ──► width = free / 2   (recompute on resize / open)
    └── customized   ──► width sticky abs   (first SetWidth from grip drag)
    │
    ▼
view_area_three_column
    show_content  → chat Fixed(width), content Fill
    !show_content → chat Fill              (unchanged)
```

No persistence. No change to when content is shown (exploration-no-tabs /
`content_collapsed` only).

## Free-space geometry

Single pure helper used by resize, open, and drag max:

```rust
// Fixed chrome outside the content↔chat split (logical px).
// Matches view: sidebar | 1px | list | 1px | content | handle | chat
fn free_content_chat_width(window_w: f32) -> f32 {
    let fixed = theme::SIDEBAR_WIDTH
        + 1.0 // sidebar_divider
        + theme::LIST_COLUMN_WIDTH
        + 1.0 // list divider
        + interaction_toggle::HANDLE_WIDTH; // export if still private
    (window_w - fixed).max(0.0)
}

fn equal_interaction_width(window_w: f32) -> f32 {
    (free_content_chat_width(window_w) / 2.0).max(MIN_PANEL_WIDTH)
}
```

`INTERACTION_COLUMN_WIDTH` (480) stops being the live default; keep only as a last-resort
seed before the first known window size, or replace default construction with
`equal_interaction_width(1200.0)` to match `.window_size((1200.0, 800.0))`.

## Interaction state

```rust
// crates/duckboard/src/area/interaction.rs
pub struct InteractionState {
    // …
    pub content_collapsed: bool,
    pub width: f32,
    /// False until the user first middle-grip drags. Session memory only.
    pub width_customized: bool,
    // …
}

// Default: width_customized = false; width = equal for initial window (or 480 seed).
```

`HandleMsg::SetWidth` path:

```rust
HandleMsg::SetWidth(w) => {
    state.width = w;
    state.width_customized = true; // first grip drag locks mode
    state.content_collapsed = false;
}
```

Toggle / collapse leave `width_customized` alone. Closing the panel does not clear
customization.

## Window size + recompute

Track logical window width on app `State` (initial `1200.0`). Subscribe with
`iced::window::resize_events()` (iced 0.14; same pattern as existing `close_requests`).

```rust
// Message
WindowResized { size: iced::Size },

// On resize / whenever an uncustomized panel should match free space:
fn rebalance_uncustomized(ix: &mut InteractionState, window_w: f32) {
    if !ix.width_customized {
        ix.width = equal_interaction_width(window_w);
    }
}
```

Call sites:

```
| Event | Action |
| --- | --- |
| Window resized | Rebalance every `InteractionState` that is uncustomized |
| Panel opens (`Toggle` → visible, or collapse forcing open) | Rebalance that panel if uncustomized |
| Content re-shown after collapse | Width already correct (sticky abs or last equal); no special case |
```

When `!show_content`, view still uses `Length::Fill` for chat; stored `width` is only the
restore target for the next split layout.

## View path

`view_area_three_column` stays fixed-width for the chat column when content is shown:

```rust
let col = if show_content {
    col.width(ix.width)          // equal or customized absolute
} else {
    col.width(Length::Fill)      // exploration-no-tabs / content_collapsed
};
```

No `FillPortion` dual-column split — avoids desync with session-bar truncation that
already budgets from `state.width`.

## Grip drag clamps

```
// interaction_toggle.rs — today
new_width = (base - dx).clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH) // max 800

// proposed
new_width = (base - dx).clamp(MIN_PANEL_WIDTH, max_panel_width)
// max_panel_width = free_content_chat_width(window_w)  (passed into the handle)
// drop MAX_PANEL_WIDTH constant for this path
```

Uncustomized 50/50 can exceed 800 on wide windows because width is set by the helper, not
the old constant. Drag max is free space (content can shrink to ~0 via drag but not invert
the row); collapse remains bottom-chevron only.

Handle API gains a `max_width: f32` (from free space). `current_width` continues to seed
drag base — must stay equal-synced while uncustomized so the first drag feels continuous.

## Impact

```
| Area | Change |
| --- | --- |
| `area/interaction.rs` | `width_customized`; SetWidth marks customized; open rebalance hook |
| `widget/interaction_toggle.rs` | Drop fixed 800 max; accept live max; min stays |
| `main.rs` | Window size state + resize subscription; rebalance; pass max into toggle view |
| `theme.rs` | Retire or demote `INTERACTION_COLUMN_WIDTH` as live default |
| Caps | New (or extended) behavioral cap for default equal split + customize-on-drag — named in `/ds-spec` |
```

No duckpond / ds / persistence / config changes.

## Decisions

- **Fixed width + recompute, not FillPortion(1)** — one source of truth for view and
  chrome (`state.width`); FillPortion would still need a numeric width for drag base and
  session-bar truncation.

- **Customized = first `SetWidth` only** — matches “first grip drag”; chevron
  toggle/collapse do not lock mode.

- **Drag max = free space, not 800** — removes the default-mode ceiling; still prevents
  negative content width.

- **Rebalance all uncustomized panels on resize** — scopes not currently visible stay
  correct when switched without a one-frame wrong width.

- **Session-only flag** — proposal non-goal; no config schema.

## Risks

- **Free-space formula drift vs real layout** (extra divider, future chrome) → equality
  off by a few px → keep constants co-located with the view geometry comment; unit-test
  the pure helper with known window widths.

- **Resize before first paint / multi-window** — app is single-window today; seed with
  `.window_size` default; resize events own thereafter.

- **Very narrow windows** — `equal` clamped to `MIN_PANEL_WIDTH` (200); content may go
  below half; acceptable and same min as drag today.
