# Rename shell to fast response

Rename obvious chrome to fast response across duckboard (module, types, fields, theme, UI
messages) and keep pure shell helpers under the new capability path.

## Tasks

- [x] 1. Rename `crates/duckboard/src/obvious_bubble.rs` → `fast_response.rs`; update
         `mod` wiring and all call sites (`ObviousChrome` → `FastResponse`, field names,
         refresh/build helpers, theme chip styles, `SendObviousAction` → activate message)

- [x] 2. Implement `visible(is_streaming, is_awaiting_user, input_empty, fr)` and keep key
         resolution, chip labels, bottom pad, and empty-send formatting as pure helpers

- [x] 3. Leave product population empty after ordinary refresh (no lifecycle chip fill)

- [x] 4. @spec chat/fast-response Ephemeral chips: Visible chips are not a stored user message

- [x] 5. @spec chat/fast-response Visibility: Idle empty composer with options shows chips

- [x] 6. @spec chat/fast-response Visibility: Streaming without awaiting user hides chips

- [x] 7. @spec chat/fast-response Visibility: Non-empty composer hides chips when not awaiting

- [x] 8. @spec chat/fast-response Visibility: Empty options hide chips

- [x] 9. @spec chat/fast-response Key resolution: Cmd-digit selects matching option

- [x] 10. @spec chat/fast-response Key resolution: Resolution is a no-op when chips not visible

- [x] 11. @spec chat/fast-response Chip labels: Option chip label is hotkey then action

- [x] 12. @spec chat/fast-response Bottom pad: Short content yields positive pad

- [x] 13. @spec chat/fast-response Bottom pad: Content at or above viewport yields zero pad

- [x] 14. @spec chat/fast-response Population: Ordinary refresh leaves options empty when not awaiting a choice

- [x] 15. @spec chat/fast-response Empty-send formatting: Bare skill name formats with leading slash

- [x] 16. @spec chat/fast-response Empty-send formatting: Already-slashed command is preserved
