# Dual-enter pure helpers

Add dual-enter display helpers and friendly enter labels in
`crates/duckboard/src/obvious_bubble.rs`, with unit tests for all Chip display scenarios.

## Tasks

- [x] 1. Implement `dual_enter_lifecycle` (true only when `lifecycle.len() > 1` and affirm
         is absent)

- [x] 2. Implement `lifecycle_friendly_name` and `lifecycle_enter_chip_label` (strip
         `/ds-` or `ds-`, title-case remainder; label is `⌘↩  <Friendly>`)

- [x] 3. @spec chat/obvious-bubble Chip display: Lifecycle chip label is hotkey then action

- [x] 4. @spec chat/obvious-bubble Chip display: Affirm chip label is hotkey then Confirm, Commit, or Create change

- [x] 5. @spec chat/obvious-bubble Chip display: Multi lifecycle without affirm dual-presents first option

- [x] 6. @spec chat/obvious-bubble Chip display: Single lifecycle does not dual-present

- [x] 7. @spec chat/obvious-bubble Chip display: Affirm present does not dual-present lifecycle

- [x] 8. @spec chat/obvious-bubble Chip display: Enter dual label is hotkey then friendly name with original send text
