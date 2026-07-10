# Chip label contrast

Raise obvious-chrome chip label contrast for light and dark themes so numbered, enter, and
reject chips stay readable without changing quiet fill tints.

## Prerequisites

- [x] @step bottom-pin-in-chat-scroll

## Context

Addresses findings in `reviews/01-followup-chip-label-contrast.md`. Labels are set in
`view_obvious_chip` (`crates/duckboard/src/widget/agent_chat.rs`) via
`theme::text_muted()` with alpha × 0.95 — too faint on the ~8% tinted chip fills.

## Tasks

- [x] 1. In `view_obvious_chip`, replace muted × 0.95 label color with a single
         higher-contrast color for all tones (prefer `theme::text_secondary()` at full
         alpha; use `text_primary` only if secondary is still weak in both themes)

- [x] 2. Keep one shared label path for Numbered, Enter, and Reject — do not per-tone text
         colors

- [x] 3. Leave chip fill styles (`chat_obvious_chip_numbered` / `_enter` / `_reject`)
         unchanged

- [x] 4. Spot-check dark and light mode: multi lifecycle chips, dual enter / Confirm, and
         Reject labels are clearly readable against chip fills
