# Surface rebound session id in ACP client

When the agent rebinds the session id during a turn (provisional open → Claude native id
after first prompt), the shared ACP client must surface the rebound id for the host to
persist.

## Prerequisites

- [x] @step defer-claude-spawn-until-first-prompt

## Context

`AcpMainRuntime` today uses only the id from `session/new`|`session/load` for
`SessionIdUpdated` and `TurnOutcome`. After step 05 the agent may return a different
`sessionId` on the `session/prompt` result; the client must prefer that for persistence
and resume.

**From step 05:** agent prompt result includes `sessionId` when provisional → native
rebind; missing-session is reported on first prompt (cold load no longer spawns). Consider
mapping prompt-time session-not-found RPC the same as load if host resume should clear the
id cleanly.

## Tasks

- [x] 1. Extend `PromptResult` in `crates/duckchat/src/acp/turn.rs` to carry an optional
         rebound `sessionId` parsed from the `session/prompt` result.

- [x] 2. In `AcpMainRuntime::run_turn`, when the prompt result rebinds the session id,
         emit `SessionIdUpdated` with the rebound id and return it in `TurnOutcome`.

- [x] 3. @spec harness/acp-client Session open and resume: When the agent rebinds the session id during a turn, the client surfaces the rebound id

- [x] 4. Run `duckchat` and `duckchat-claude-acp` tests and fix regressions from the
         rebind path.
