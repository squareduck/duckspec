# Numbered tone and chrome view

Add quiet light-blue numbered chip styling, pure bottom-pad math, and rework
`view_obvious_chrome` for the dual-enter tone matrix.

## Prerequisites

- [x] @step dual-enter-pure-helpers

## Tasks

- [x] 1. Implement `chrome_bottom_pad(viewport_h, content_h, prev_pad)` as
         `max(0, viewport_h - (content_h - prev_pad))` in `obvious_bubble.rs`

- [x] 2. @spec chat/obvious-bubble Chrome bottom pad: Short content yields positive pad

- [x] 3. @spec chat/obvious-bubble Chrome bottom pad: Content at or above viewport yields zero pad

- [x] 4. Add `chat_obvious_chip_numbered` in `theme.rs` (~8% `accent()` mix from neutral
         base, same recipe as enter/reject)

- [x] 5. Rework `view_obvious_chrome` / `ObviousChipTone`: multi no-affirm → all lifecycle
         Numbered (blue) plus dual Enter row with `lifecycle_enter_chip_label` and
         original send text; multi/affirm → Numbered lifecycle + green/red gate; single
         option → one Enter (green) chip
