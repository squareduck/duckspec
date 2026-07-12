# Window resize and view wiring

Track window size, rebalance on resize/open, drag max = free space, keep content-hidden
Fill; cover the fill scenario.

## Prerequisites

- [x] @step geometry-and-width-mode

## Tasks

- [x] 1. Track logical window width on app `State` (seed with default window size); handle
         `WindowResized` via `iced::window::resize_events()`

- [x] 2. On resize, call `rebalance_uncustomized` for every interaction panel; on panel
         open, rebalance that panel when uncustomized

- [x] 3. Pass free-space max into the interaction handle; drop the fixed 800 max clamp so
         uncustomized half and drag can use full free space (min panel width remains)

- [x] 4. Keep `view_area_three_column` Fixed(width) when content is shown and Fill when
         content is hidden; demote or remove live use of `INTERACTION_COLUMN_WIDTH` as the
         default

- [x] 5. @spec layout/content-chat-split Content-hidden fill: Interaction column fills when content column is hidden

- [x] 6. Smoke-check three-column areas: uncustomized equal split on resize, first grip
         lock, exploration-no-tabs and manual collapse still fill
