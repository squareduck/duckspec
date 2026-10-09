# Match the hot Claude process to model and effort

The inner `claude` process is reused only while its model and effort still match the
prompt.

## Prerequisites

- [x] @step carry-effort-on-the-acp-wire

## Context

`TurnRequest.effort: Option<String>` is already copied onto the main path's
`session/prompt` as `effort`. `reasoningEffort` stays the reasoning-mode field. Oneshot
`AcpTurn::prompt` does not send `effort`. The hot Claude process does not yet record the
model or effort it was spawned with.

## Tasks

- [x] 1. Record model and effort on the hot process, and pass `--effort` from
         `crates/duckchat-claude-acp/src/claude/spawn.rs` only when the prompt has a
         level.

- [x] 2. On a model or effort mismatch, kill the process and cold-resume the same native
         id. Do not cancel an in-flight turn when the picker changes. Keep title and reply
         oneshots off that process.

- [x] 3. @spec harness/claude Duplex main heat: Hot process follows the prompt's model and effort

- [x] 4. @spec harness/claude Duplex main heat: A model or effort change during an in-flight turn does not cancel that turn

- [x] 5. @spec harness/claude Duplex main heat: A title or reply oneshot leaves the main hot process in place
