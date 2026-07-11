# Unify agent ACP stdout writer

Use one stdout write path for mid-turn `session/update` notifications and prompt results
so the agent does not mix tokio and `std::io` writers on the same stream.

## Prerequisites

- [x] @step package-agent-with-duckboard-and-document-install

## Context

Review finding 2: `main.rs` uses `tokio::io::stdout` for results and `std::io::stdout` for
live updates. Keep progressive streaming from step 07.

## Tasks

- [x] 1. Refactor `crates/duckchat-claude-acp/src/main.rs` so all ACP JSON-RPC lines use a
         single writer (e.g. async channel + select, or one shared async path).

- [x] 2. Preserve live `session/update` delivery before the prompt result.

- [x] 3. Run `duckchat-claude-acp` tests and fix regressions.
