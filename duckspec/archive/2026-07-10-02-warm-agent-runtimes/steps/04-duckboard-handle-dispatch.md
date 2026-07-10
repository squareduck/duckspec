# duckboard handle dispatch

Route title summary and reply suggestions through the chat `AgentHandle` so duckboard no
longer cold-constructs providers per oneshot.

## Prerequisites

- [ ] @step worker-dual-path-and-handle-oneshots

## Context

`AgentHandle::title_summary` and `AgentHandle::reply_suggestions` are live (async, Clone
handle, empty-assistant short-circuit for replies). Call them from `Task::perform` after
`TurnComplete`; drop the harness-matched cold provider oneshots in `main.rs`.

## Tasks

- [x] 1. In `crates/duckboard/src/main.rs`, after `TurnComplete`, call
         `handle.title_summary(...)` / `handle.reply_suggestions(...)` via `Task::perform`
         instead of matching harness and building `GrokProvider` / `ClaudeCodeProvider`.

- [x] 2. Drop the oneshot harness `match` arms and any now-unused imports; keep harness
         selection only for the agent subscription / model pin path.

- [x] 3. Confirm `AgentHandle` is `Clone` and safe to move into the async task (same as
         today’s handle use for turns); handle missing handle / superseded `prompts_gen`
         as today.

- [x] 4. `cargo test -p duckboard -p duckchat` and fix compile fallout from the Provider /
         handle API change.
