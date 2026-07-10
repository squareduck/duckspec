# Chat config flags

Add global `ChatConfig` with agent input hints off and auto messages on by default.

## Tasks

- [x] 1. In `crates/duckboard/src/config.rs`, add `ChatConfig` with
         `agent_input_hints: bool` and `auto_messages: bool`; nest it on `Config` as
         `chat` with `#[serde(default)]`

- [x] 2. Implement `Default` so `agent_input_hints` is `false` and `auto_messages` is
         `true`

- [x] 3. @spec chat/default-prompts Agent input hints gate: Default agent input hints setting is disabled

- [x] 4. @spec chat/obvious-bubble Chrome visibility: Default auto messages setting is enabled
