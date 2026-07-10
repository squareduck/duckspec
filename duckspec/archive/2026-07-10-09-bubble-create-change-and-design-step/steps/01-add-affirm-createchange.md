# Add Affirm CreateChange

Add `Affirm::CreateChange` with literal send text `Create change` and cover the chip-label
scenario in the pure chrome helpers.

## Tasks

- [x] 1. Add `Affirm::CreateChange` in `crates/duckboard/src/obvious_bubble.rs`;
         `send_text()` returns `"Create change"`; update `ObviousChrome` / `Affirm` docs
         to mention Create change (nonempty exploration)

- [x] 2. Extend unit tests so chip label / send text cover Create change (⌘↩ prefix +
         exact action string)

- [x] 3. @spec chat/obvious-bubble Chip display: Affirm chip label is hotkey then Confirm, Commit, or Create change
