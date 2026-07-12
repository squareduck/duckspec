# Landmark keybinds and scroll

Wire ⌘↑/↓/←/→ when chat is active: history ends, Answer jumps via existing block widget
ids, stick-to-bottom rules, and modal yield.

## Prerequisites

- [x] @step answer-reply-and-viewport-helpers

## Tasks

- [x] 1. Add `ChatLandmarkAction` and `keybind_chat_landmarks` in
         `crates/duckboard/src/keybinds.rs` (chat tab active + session; not
         terminal-focused)

- [x] 2. Dispatch ⌘↑/↓/←/→ in `main` after modal handlers; leave bare arrows to the
         composer

- [x] 3. History top: scroll to y=0, clear stick-to-bottom and pending snap, set
         `chat_scroll_overridden`

- [x] 4. History bottom: `snap_to_end` and set stick-to-bottom

- [x] 5. Prev/next: resolve current Answer (viewport or stick), jump with
         `find::scroll_block_to_top` + `chat_block_widget_id`, clear stick on leave-bottom
         jumps (measure Answer tops via layout Operation when needed)

- [x] 6. Smoke the full path in the app: band on latest Answer, ⌘ landmarks with composer
         focused, modals still own their keys

## Outcomes

- Live visual smoke (band contrast strength, ⌘-arrow feel with composer focused) is worth
  a quick pass in duckboard; unit tests cover pure helpers and the binary builds clean.
