# Helpers and equal-width tests

Add `for_window` and `show_panel` on the interaction panel, and cover both new equal-width
scenarios in unit tests.

## Tasks

- [x] 1. Add `InteractionState::for_window(window_w)` in
         `crates/duckboard/src/area/interaction.rs` — same fields as `Default`, width from
         `equal_interaction_width(window_w)`

- [x] 2. Add `show_panel(ix, window_w)` — set `visible = true` and call
         `rebalance_uncustomized`

- [x] 3. Keep `Default` on `DEFAULT_WINDOW_WIDTH`; leave door-open `just_opened` rebalance
         as-is

- [x] 4. @spec layout/content-chat-split Uncustomized equal width: Panel created for a known window starts at half free space

- [x] 5. @spec layout/content-chat-split Uncustomized equal width: Programmatic open rebalances to half free space
