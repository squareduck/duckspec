# Drop cancel chrome

Remove cancel chip, `cancel` shell field, ⌘⌫ binding/view, and obsolete cancel `@spec`
tests/step tasks. Esc and freeform-while-awaiting still complete parked choices as
cancelled on the wire.

## Tasks

- [x] 1. Drop `cancel` / `FastResponsePick::Cancel` / `resolve_cmd_backspace` /
         `cancel_chip_label` from `crates/duckboard/src/fast_response.rs`; options-only
         `is_empty` / `from_user_choice`

- [x] 2. Stop painting cancel chips in `crates/duckboard/src/widget/agent_chat.rs`; drop
         Cmd-Backspace wiring in keyboard handling

- [x] 3. Stop passing / honoring `allow_cancel` for shell fill (event may still carry it;
         UI ignores)

- [x] 4. Remove cancel-only unit tests and their `@spec` comments in
         `fast_response.rs` (and call-site tests that assert cancel)

- [x] 5. Remove obsolete cancel `@spec` tasks from steps
         `01-rename-shell-to-fast-response.md` and `05-duckboard-choice-wiring.md` so
         audit resolves
