# Chat cancel resync

When a turn is cancelled, the answer draft the user kept seeing is recorded on the session
as unsynced and appended to the next send as agent-facing context, so the agent's own
history and the visible transcript stay in agreement.

## Requirement: Draft capture on cancellation

When a turn ends by cancellation — user cancel or answer-thrash trip — the session SHALL
record the answer text the transcript keeps for that turn as its unsynced draft, including
deltas that arrive between the cancel request and the turn's end. Cancellation with an
empty in-flight draft SHALL leave no unsynced draft: text already committed at tool
boundaries is recorded by the agent runtime and needs no resync.

> test: code

### Scenario: Thrash trip captures the kept draft

- **GIVEN** a streaming turn whose in-flight answer draft is non-empty
- **WHEN** the answer-thrash budget trips and the turn is cancelled
- **THEN** the session's unsynced draft equals the kept draft

> test: code
> - crates/duckboard/src/area/interaction.rs:1813

### Scenario: User cancel captures the in-flight draft

- **GIVEN** a streaming turn whose in-flight answer draft is non-empty
- **WHEN** the user cancels the turn
- **THEN** the session's unsynced draft equals that draft

> test: code
> - crates/duckboard/src/area/interaction.rs:1833

### Scenario: Deltas arriving after cancel are part of the captured draft

- **GIVEN** a streaming turn cancelled by the user with a non-empty in-flight answer draft
- **WHEN** further answer deltas arrive before the turn ends
- **THEN** the session's unsynced draft includes those deltas

> test: code
> - crates/duckboard/src/area/interaction.rs:1869

### Scenario: Cancellation with no in-flight draft records nothing

- **GIVEN** a streaming turn whose answer text was committed at a tool boundary
- **AND** no answer text has streamed since
- **WHEN** the turn is cancelled
- **THEN** the session has no unsynced draft

> test: code
> - crates/duckboard/src/area/interaction.rs:1852

## Requirement: Resync reminder on next send

The next prompt sent on a session holding an unsynced draft SHALL carry that draft after
the user's text, identified as the user-visible reply of an interrupted turn; the user's
text SHALL precede the reminder in the outgoing prompt. Riding a send SHALL clear the
unsynced draft. A recovery send that already carries the transcript history in its prompt
SHALL clear the unsynced draft without adding a reminder.

> test: code

### Scenario: The next send carries the draft after the user's text

- **GIVEN** a session holding an unsynced draft
- **WHEN** a prompt is sent on that session
- **THEN** the user's text precedes the unsynced draft in the outgoing prompt

> test: code
> - crates/duckboard/src/area/interaction.rs:1896

### Scenario: The reminder rides only one send

- **GIVEN** a session holding an unsynced draft
- **WHEN** two prompts are sent in sequence on that session
- **THEN** only the first outgoing prompt carries the draft

> test: code
> - crates/duckboard/src/area/interaction.rs:1916

### Scenario: A recovery resend carrying transcript history clears the draft without a reminder

- **GIVEN** a session holding an unsynced draft
- **WHEN** a recovery send carries the transcript history in its prompt
- **THEN** the session afterward holds no unsynced draft
- **AND** the recovery prompt carries no resync reminder

> test: code
> - crates/duckboard/src/area/interaction.rs:1933
