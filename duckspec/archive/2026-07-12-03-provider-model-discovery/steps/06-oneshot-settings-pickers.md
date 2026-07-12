# Oneshot settings pickers

Show per-harness oneshot model pickers in Settings only when agent input hints are
enabled, backed by the process catalog and global `oneshot_models` config.

## Prerequisites

- [x] @step oneshot-model-resolution-and-worker-wiring

## Tasks

- [x] 1. In `crates/duckboard/src/area/settings.rs` Chat section, when `agent_input_hints`
         is on, render one oneshot model picker per harness with a non-empty catalog slice

- [x] 2. Persist picker selection into `config.chat.oneshot_models[harness]`; hide pickers
         when hints are off

- [x] 3. @spec chat/oneshot-models Settings pickers when hints enabled: With agent input hints on, each harness with catalog models offers an oneshot model picker

- [x] 4. @spec chat/oneshot-models Settings pickers when hints enabled: With agent input hints off, oneshot model pickers are not shown
