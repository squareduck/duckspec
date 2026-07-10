# Settings and session wire-up

Expose both toggles in Settings and thread flags through view, empty Enter, Tab, and
chrome activation so pure rules apply live.

## Prerequisites

- [x] @step effective-input-hints-pure
- [x] @step oneshot-launch-gate
- [x] @step auto-messages-visibility

## Context

`AgentSession::session_input_hints(agent_input_hints)` already exists (step 02) and is
used in view / Enter / Tab with a hardcoded `true`. Replace those call sites with
`config.chat.agent_input_hints`. Task 2 is then “thread config into the helper calls”
rather than inventing the helper again.

`chrome_visible` and `resolve_cmd_*_when_visible` take `auto_messages` (step 04). Call
sites currently pass `true` (including `agent_chat::view`). Thread
`config.chat.auto_messages` through view, key path, and `SendObviousAction`.

## Tasks

- [x] 1. In `crates/duckboard/src/area/settings.rs`, add a Chat section with toggles for
         agent input hints and auto messages; save via `config::save` on change

- [x] 2. Add a `session_input_hints` (or equivalent) helper in `area/interaction.rs` that
         builds the effective list from session emptiness, `obvious_chrome.lifecycle[0]`,
         oneshot storage, and `config.chat.agent_input_hints`

- [x] 3. Use that helper in chat view, empty `SendPressed`, and `CycleDefaultPrompt` so
         empty-session seed and agent lists share one path

- [x] 4. Pass `config.chat.auto_messages` into chrome visibility and key-resolution call
         sites (including `SendObviousAction` re-check)

- [x] 5. Confirm Settings toggles apply without restart (next view/update reflects flags)

- [x] 6. Smoke: empty exploration session shows under-input `/ds-explore` with Enter send;
         agent hints off skips oneshot after a turn; auto messages off hides all chips
