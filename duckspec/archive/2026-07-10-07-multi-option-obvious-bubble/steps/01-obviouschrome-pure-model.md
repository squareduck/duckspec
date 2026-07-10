# ObviousChrome pure model

Replace single-command helpers in `crates/duckboard/src/obvious_bubble.rs` with
`ObviousChrome`, format/visibility/key-resolution/label helpers, and unit tests for the
non-UI contracts.

## Tasks

- [x] 1. Introduce `ObviousChrome`, `Affirm`, `format_lifecycle_command`,
         `chrome_is_empty`, `chrome_visible`, `resolve_cmd_enter`,
         `resolve_cmd_backspace`, `resolve_cmd_digit`, and chip label helpers per design;
         remove or rewrite old single-command APIs that no longer match

- [x] 2. Keep empty-send formatting usable by soft-hint callers (first lifecycle option)
         without seeding the oneshot composer list

- [x] 3. @spec chat/obvious-bubble Lifecycle option formatting: Bare skill name formats with leading slash

- [x] 4. @spec chat/obvious-bubble Lifecycle option formatting: Already-slashed command is preserved

- [x] 5. @spec chat/obvious-bubble Chrome visibility: Idle empty composer with chrome shows chrome

- [x] 6. @spec chat/obvious-bubble Chrome visibility: Streaming hides chrome

- [x] 7. @spec chat/obvious-bubble Chrome visibility: Non-empty composer hides chrome

- [x] 8. @spec chat/obvious-bubble Chrome visibility: Empty chrome is hidden

- [x] 9. @spec chat/obvious-bubble Chrome visibility: Oneshot pending does not hide chrome when otherwise visible

- [x] 10. @spec chat/obvious-bubble Key resolution: Cmd-Enter sends affirm when present

- [x] 11. @spec chat/obvious-bubble Key resolution: Cmd-Enter sends first lifecycle when affirm absent

- [x] 12. @spec chat/obvious-bubble Key resolution: Cmd-Backspace sends Reject when decline set

- [x] 13. @spec chat/obvious-bubble Key resolution: Cmd-digit sends matching lifecycle option

- [x] 14. @spec chat/obvious-bubble Key resolution: Resolution is a no-op when chrome not visible

- [x] 15. @spec chat/obvious-bubble Key resolution: Resolved text ignores oneshot list when both differ

- [x] 16. @spec chat/obvious-bubble Chip display: Lifecycle chip label is hotkey then action

- [x] 17. @spec chat/obvious-bubble Chip display: Affirm chip label is hotkey then Confirm or Commit

- [x] 18. @spec chat/obvious-bubble Ephemeral chrome: Visible chrome is not a stored user message
