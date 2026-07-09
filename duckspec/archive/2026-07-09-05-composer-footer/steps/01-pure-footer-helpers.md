# Pure footer helpers

Extract pure predicates and formatters for the composer footer behavioral rules and cover
every `chat/composer-footer` scenario with unit tests.

## Context

No design doc. Land helpers next to the existing meta-row logic in
`crates/duckboard/src/widget/agent_chat.rs` (where `context_fill`, `group_choices`, and
`format_number` already live).

- **Resend hint:** a pure `show_resend_history_hint(will_resume, has_messages)` (or
  equivalent) — true only when `!will_resume && has_messages`.

- **Usage readout:** format from known fill; below 75% percentage only; at or above 75%
  include used, max, and percentage (reuse `context_fill` + `format_number`). Unknown
  window is out of scope here (model-picker).

- **Closed model label:** menu choices must stay harness-prefixed (`Harness · Display`)
  for `harness/model-picker`. The closed control needs a short display name only — prefer
  a separate field or builder (e.g. `closed_label` / display-only string) rather than
  stripping the menu label, so the existing grouped-choices test keeps passing.

Tests go in the existing `#[cfg(test)] mod tests` in `agent_chat.rs`, with single-line
`@spec chat/composer-footer …` comments above each test.

## Tasks

- [x] 1. Implement pure helpers for resend visibility, progressive usage formatting, and
         short closed model label (menu labels remain harness-prefixed)

- [x] 2. @spec chat/composer-footer Resend hint only when history would be resent: Hint shown when history would be resent

- [x] 3. @spec chat/composer-footer Resend hint only when history would be resent: Hint hidden when next send would resume

- [x] 4. @spec chat/composer-footer Resend hint only when history would be resent: Hint hidden when transcript is empty

- [x] 5. @spec chat/composer-footer Progressive usage readout: Cool fill shows percentage only

- [x] 6. @spec chat/composer-footer Progressive usage readout: Hot fill shows used, max, and percentage

- [x] 7. @spec chat/composer-footer Short closed model label: Closed label is the model display name
