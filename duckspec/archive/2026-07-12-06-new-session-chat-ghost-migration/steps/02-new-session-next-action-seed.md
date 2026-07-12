# New-session next-action seed

Shared helper to seed inheritance from the active donor on change multi-session
`NewSession`; wire change and ideas handlers.

## Prerequisites

- [x] @step inherited-next-action-list-resolution

## Tasks

- [x] 1. Add `new_session_with_inherited_next_actions` on `InteractionState` /
         `interaction` module: clone active donor `next_actions` when non-empty and
         `scope_kind == ScopeKind::Change`, set `inherited_next_actions`,
         `refresh_next_actions(true)` (active index 0), return empty `AgentSession`

- [x] 2. Wire `Msg::NewSession` in `crates/duckboard/src/area/change.rs` and multi-session
         path in `crates/duckboard/src/area/ideas.rs` to use the helper while the donor is
         still `active` (before `insert(0)`)

- [x] 3. @spec chat/default-prompts New-session next-action inheritance: New change session inherits active session next actions

- [x] 4. @spec chat/default-prompts New-session next-action inheritance: New change session with empty donor keeps bootstrap behavior

- [x] 5. @spec chat/default-prompts New-session next-action inheritance: Inherited list starts at first action
