# Inherited next-action list resolution

Extend pure list building and `AgentSession` so empty sessions prefer a sticky inherited
list over lifecycle bootstrap, and clear inheritance when non-empty.

## Tasks

- [x] 1. Add `inherited_next_actions: Option<Vec<NextAction>>` on `AgentSession`
         (`crates/duckboard/src/area/interaction.rs`); default `None` in `from_session`;
         document as ephemeral and not persisted

- [x] 2. Extend `default_prompts::next_action_list` with
         `inherited: Option<&[NextAction]>` — empty + non-empty inherited wins over
         bootstrap; update all call sites and existing unit tests for the new parameter

- [x] 3. In `AgentSession::refresh_next_actions`, pass inherited when empty; when
         non-empty clear `inherited_next_actions` and rebuild from trailing next only

- [x] 4. @spec chat/default-prompts Next-action list: Empty session with inherited next actions uses inherited list

- [x] 5. @spec chat/default-prompts Next-action list: Empty session without inherited falls back to lifecycle

- [x] 6. @spec chat/default-prompts Next-action list: Non-empty session drops inheritance
