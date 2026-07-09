# Wire meta strip UI

Hook the pure footer helpers into the chat input meta row and lighten the model control so
the strip stays paper-blended.

## Prerequisites

- [x] @step pure-footer-helpers

## Context

View construction lives in `agent_chat::view` (meta row under the prompt). Status is built
in `crates/duckboard/src/area/interaction.rs` (`StatusInfo` / `will_resume`).

Helpers from step 01 (in `agent_chat.rs`):

- `show_resend_history_hint(will_resume, has_messages)`

- `format_usage_readout(tokens, window)` — known window only; cool → `%`, hot →
  `used / max (%)`

- `ModelChoice.closed_label` — short display name; `ModelChoice.label` remains
  harness-prefixed for the menu. `Display` still writes `label`, so the closed control
  keeps the long form until this step switches it (selected entry uses `closed_label` for
  display while menu options keep `label`, or equivalent).

Wire-up:

- Drive the resend hint from the pure predicate using `will_resume` and whether
  `session.messages` is non-empty (pass a flag on `StatusInfo` or compute in the view from
  session).

- Format the usage string with the progressive helper instead of always emitting
  `used / max (%)`.

- Closed pick-list display must use `closed_label`; open menu options keep
  harness-prefixed `label`.

- Visual only (no new specs): reduce pick-list chrome (padding / style) so the control
  reads as lightweight text+chevron on the paper surface; no extra border around the meta
  strip.

## Tasks

- [x] 1. Wire resend-hint visibility in the meta row from the pure predicate and
         transcript emptiness

- [x] 2. Wire progressive usage formatting into the meta-row token readout

- [x] 3. Show the short closed model label on the model control while keeping
         harness-prefixed menu choices

- [x] 4. Lighten model-control chrome so it blends into the paper input (no new strip
         border)

- [x] 5. Run `cargo test -p duckboard` and fix any regressions (including existing
         model-picker tests)
