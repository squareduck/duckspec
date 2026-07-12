# Programmatic chat split width - Design

Make every force-visible interaction panel and every new panel construction share the same
live-window equal-split path door open already uses.

## Approach

Reuse `equal_interaction_width` / `rebalance_uncustomized` — no new geometry. Close two
holes:

1. **Construction** — new panels default to half of `DEFAULT_WINDOW_WIDTH` (1200), not
   live `window_width`

2. **Programmatic show** — `visible = true` outside the door path never rebalances

```
                    equal_interaction_width(window_w)
                              │
          ┌───────────────────┼───────────────────┐
          ▼                   ▼                   ▼
   for_window(w)     rebalance on show     rebalance on door open
   (new panel)       (show_panel)          (just_opened — keep)
          │                   │                   │
          └───────────────────┴───────────────────┘
                              │
                    ix.width (uncustomized only)
```

Door open (`update` → `just_opened` → `rebalance_uncustomized`) stays as-is. Production
force-show sites go through one helper so they cannot skip rebalance.

## Interaction width helpers

In `crates/duckboard/src/area/interaction.rs`:

```rust
impl InteractionState {
    /// Same as Default, but equal width from live window free space.
    pub fn for_window(window_w: f32) -> Self { /* … */ }
}

/// Force-open (or no-op if already visible): set visible and rebalance if uncustomized.
pub fn show_panel(ix: &mut InteractionState, window_w: f32) {
    ix.visible = true;
    rebalance_uncustomized(ix, window_w);
}
```

- `Default` keeps `equal_interaction_width(DEFAULT_WINDOW_WIDTH)` for tests and boot
  before a real size is known

- `for_window` is the production constructor when `window_w` is in hand

- `show_panel` is the only force-visible API; `rebalance_uncustomized` already no-ops when
  `width_customized`

Door path keeps calling `rebalance_uncustomized` on `just_opened` (including collapse that
opens the panel). No need to route door through `show_panel`.

## Construction call sites

Prefer live width when inserting scope panels:

```
| Site | Today | Target |
| --- | --- | --- |
| `ideas::open_idea` | `entry.or_default()` | `or_insert_with(\|\| for_window(window_w))` |
| `change::update` SelectChange / AddExploration | `or_default()` | `for_window` when inserting |
| Caps/Codex `entry.or_default()` in main | default | `or_insert_with(\|\| for_window(state.window_width))` where entry is used |
| `State::new` / `open_project` Caps+Codex seed | `default()` | `for_window(self.window_width)` (same as default until first resize; consistent API) |
| Unit/integration seeds | `default()` | leave (or set width in test when asserting layout) |
```

`open_idea` gains `window_w: f32` (already on `ideas::update`).

## Programmatic show call sites

Replace bare `ix.visible = true` on production force-open paths with
`show_panel(ix, window_w)`:

```
| Path | File (approx) |
| --- | --- |
| Idea open with exploration/change scope | `area/ideas.rs` `open_idea` |
| Select change / exploration | `area/change.rs` SelectChange |
| Add exploration | `area/change.rs` AddExploration |
```

Door `Toggle` / `SetCollapsed` stay in `interaction::update` (return `just_opened`).

## Spec surface

Extend existing cap `layout/content-chat-split` (not a new capability):

- Uncustomized equal width also applies when the panel is force-shown without the door
- New panel construction with a known window uses that window for the initial equal half

Scenarios should cover Explore / `open_idea`-style force-show and `for_window` vs default
construction. Implementation details (helper names) stay out of the requirement text.

## Impact

- Duckboard only: `interaction.rs`, `ideas.rs`, `change.rs`, small main Caps/Codex entry
  sites

- No schema, persistence, or harness changes

- Spec/doc delta under `duckspec/caps/layout/content-chat-split/`

- Existing unit tests that assume default width still pass; add scenarios for live-window
  force-show

## Decisions

- **Helper vs scatter rebalance** — `show_panel` + `for_window`. Alternative: only
  rebalance at each call site (rejected: easy to miss the next force-open).

- **Keep `Default` on default window** — Alternative: remove Default and require window
  everywhere (rejected: noisy for tests and boot).

- **Extend `content-chat-split`** — Alternative: new cap under ideas (rejected: geometry
  is layout-owned; Explore is only the sharpest call site).

- **Door path unchanged** — Alternative: unify door open through `show_panel` (optional
  later; not required for intent).

## Risks

- **Missed `visible = true`** → grep for production assignments; route only force-open
  paths through `show_panel`

- **Double rebalance on door + show** → harmless while uncustomized; customized still
  protected

- **`or_default` left on a hot path** → new panels still fixed by `show_panel` when
  force-shown; door open still rebalances; `for_window` is defense in depth
