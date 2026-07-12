# Duckboard registration and packaging

Register the openai-codex provider in the model catalog and harness dispatch, and ship
`duckchat-codex-acp` next to duckboard like the Claude agent.

## Prerequisites

- [x] @step thin-openai-codex-provider

## Tasks

- [x] 1. Register openai-codex in `crates/duckboard/src/agent.rs` (catalog refresh,
         harness rank, `Harness::dispatch` / `drive_provider`)

- [x] 2. Bundle and install: copy `duckchat-codex-acp` sibling to duckboard (`just bundle`
         / install paths, README notes as needed)

- [x] 3. Smoke: model catalog includes openai-codex when agent + codex available; dispatch
         runs turns on that provider

- [x] 4. Verify Claude and Grok registration and dispatch remain unchanged

- [x] 5. Document env `DUCKCHAT_CODEX_ACP` and sibling binary discovery for local dev
