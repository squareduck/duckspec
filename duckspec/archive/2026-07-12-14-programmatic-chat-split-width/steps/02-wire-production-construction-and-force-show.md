# Wire production construction and force-show

Route production panel creation through `for_window` and force-show through `show_panel`
so Explore and other opens match door-open equal split.

## Prerequisites

- [x] @step helpers-and-equal-width-tests

## Tasks

- [x] 1. Construction: use `for_window(window_w)` / `or_insert_with` for idea open, change
         select, AddExploration, Caps/Codex entry, and Caps/Codex seed in `State::new` /
         `open_project` (`ideas.rs`, `change.rs`, `main.rs` per design)

- [x] 2. Pass `window_w` into `open_idea` from `ideas::update`

- [x] 3. Force-show: replace production `ix.visible = true` with
         `show_panel(ix, window_w)` on idea open, SelectChange, and AddExploration

- [x] 4. Grep for remaining production `visible = true` on interaction panels; leave door
         path and tests alone
