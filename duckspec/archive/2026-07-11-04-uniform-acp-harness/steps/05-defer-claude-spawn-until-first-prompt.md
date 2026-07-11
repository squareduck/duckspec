# Defer Claude spawn until first prompt

Stop the Claude hang: `session/new` and cold `session/load` must not start the official
`claude` process. The first `session/prompt` spawns duplex, writes user content, binds
Claude's native session id, and returns it on the prompt result.

## Prerequisites

- [x] @step flip-claude-provider-and-host-cleanup

## Context

Followup `01-followup-claude-hang-defer-spawn-to-first-prompt`: live `claude -p` duplex
emits no session id until a user stream-json line is written. Waiting for init in
`spawn_and_init` before any write hangs every Claude turn. Specs under `harness/claude`
Session lifecycle require open without starting `claude` and native id after the first
prompt.

## Tasks

- [x] 1. Change `session_new` in `crates/duckchat-claude-acp/src/agent.rs` to return a
         provisional ACP session id without spawning Claude; store pending open state
         (cwd, model) for the first prompt.

- [x] 2. Change cold `session_load` to record the resume id without spawning; when already
         duplex-hot for that id, keep reusing heat as today.

- [x] 3. Add a duplex first-open path that spawns Claude, writes the user content, then
         reads init and the rest of the stream (no `wait_for_session_id` before write) in
         `crates/duckchat-claude-acp/src/claude/duplex.rs`.

- [x] 4. Update `run_prompt` to spawn/bind via that path when cold, rebind the live
         session id to Claude's native id, and include `sessionId` on the prompt result
         when the id differs from open (for the client rebind step).

- [x] 5. Rewrite agent unit tests that assumed spawn-at-open (session lifecycle + duplex
         heat) so they match defer-spawn while staying green.

- [x] 6. @spec harness/claude Session lifecycle and native session ids: Opening a new session does not start the official claude process before the first user prompt
