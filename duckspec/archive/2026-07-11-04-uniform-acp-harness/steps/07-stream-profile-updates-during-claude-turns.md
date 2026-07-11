# Stream profile updates during Claude turns

Stop batching Claude profile updates until the turn ends. Emit each `session/update` to
the ACP parent as Claude lines arrive; send the prompt result only at stop so the host can
stream like Grok.

## Prerequisites

- [x] @step surface-rebound-session-id-in-acp-client

## Context

Followup `02-followup-claude-progressive-stream-and-thinking`: `main.rs` waits for
`run_prompt` to return a full `Vec` of updates, then flushes them. Shared client and
duckboard already progressive-render live notifications.

## Tasks

- [x] 1. Change duplex `prompt` / `open_with_first_prompt` in
         `crates/duckchat-claude-acp/src/claude/duplex.rs` to accept an update sink and
         invoke it for each mapped profile update as lines arrive (not only via a returned
         `Vec`).

- [x] 2. Update agent `run_prompt` (and callers) to forward that sink so updates can leave
         the process mid-turn.

- [x] 3. Rewrite `session/prompt` handling in `crates/duckchat-claude-acp/src/main.rs` to
         write `session/update` notifications live during the turn, then write the prompt
         result at stop.

- [x] 4. Add a test that observes a profile `session/update` before the `session/prompt`
         result is returned (scripted Claude peer that streams then ends).

- [x] 5. @spec harness/claude Profile-compatible event emission: Profile updates are delivered to the host before the turn completes
