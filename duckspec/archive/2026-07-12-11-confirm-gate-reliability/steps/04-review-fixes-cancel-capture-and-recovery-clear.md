# Review fixes cancel capture and recovery clear

Close review findings 1-2: late post-cancel deltas join the captured draft via a
cancel-in-flight re-capture at turn end, and the resume-loss recovery resend clears the
unsynced draft its history preamble already carries.

## Context

Guard the re-capture against stale state: a cancel press with no turn running must not
make the *next completed* turn's draft look cancelled. Reset the cancel-in-flight flag
wherever a new turn starts (`send_prompt_text`), mirroring `reset_answer_thrash`.

## Tasks

- [x] 1. Add a `cancel_in_flight` flag to `AgentSession`
         (`crates/duckboard/src/area/interaction.rs`): set in the `CancelPressed` arm,
         reset in `send_prompt_text`; in the `TurnComplete` arm
         (`crates/duckboard/src/main.rs`) re-run `capture_unsynced_draft` before
         `flush_all_pending` when the flag is set, then clear it

- [x] 2. In the resume-loss recovery resend (`crates/duckboard/src/area/interaction.rs`,
         re-dispatch after lost agent session), clear `unsynced_draft` before its
         `save_session` — the history preamble already carries the kept draft as a
         committed message; extract a small seam if needed for the test

- [x] 3. @spec chat/cancel-resync Draft capture on cancellation: Deltas arriving after cancel are part of the captured draft

- [x] 4. @spec chat/cancel-resync Resync reminder on next send: A recovery resend carrying transcript history clears the draft without a reminder
