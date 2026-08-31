# @ Chat cancel resync

## @ Requirement: Draft capture on cancellation

When a turn ends by user cancellation, the session SHALL record the answer text the
transcript keeps for that turn as its unsynced draft, including deltas that arrive between
the cancel request and the turn's end. Cancellation with an empty in-flight draft SHALL
leave no unsynced draft: text already committed at tool boundaries is recorded by the
agent runtime and needs no resync.

### - Scenario: Thrash trip captures the kept draft
