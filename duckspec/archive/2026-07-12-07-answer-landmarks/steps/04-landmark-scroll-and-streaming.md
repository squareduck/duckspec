# Landmark scroll and streaming

Make all four ⌘-arrow landmarks take effect once (especially ⌘↓ on restored sessions) and
keep working while a turn is streaming.

## Context

From `reviews/01-followup-landmark-scroll-and-band-contrast.md`: HistoryBottom omits
`chat_scroll_overridden` so scroll-preservation can undo `snap_to_end`; streaming makes
all landmarks a no-op (stick/snap fight and/or Captured keys).

## Tasks

- [x] 1. Set `chat_scroll_overridden` on HistoryBottom in `apply_chat_landmark` (match
         HistoryTop / prev / next)

- [x] 2. Audit all landmark arms against `update_with_scroll_preservation` so one keypress
         is not replayed away

- [x] 3. While streaming, ensure landmark jumps win over stick-to-bottom auto-snap for
         that intent (leave-bottom unsticks; bottom re-sticks without fighting the jump)

- [x] 4. If ⌘-arrows never reach `KeyPress` mid-stream (iced `Captured`), route them so
         landmarks still dispatch when chat is active

- [x] 5. Smoke: restored long session one ⌘↓ to end; mid-stream all four ⌘-arrows respond

## Outcomes

- All landmark arms set `chat_scroll_overridden` up front.

- Leave-bottom clears stick so StreamTick stops auto-snapping; prev/next materialize dirty
  UI first so anchors exist.

- `handle_key_event` forwards Captured ⌘-arrows so TextEdit caret shortcuts do not swallow
  landmarks.

- Interactive smoke still worth a quick pass in duckboard after rebuild.
