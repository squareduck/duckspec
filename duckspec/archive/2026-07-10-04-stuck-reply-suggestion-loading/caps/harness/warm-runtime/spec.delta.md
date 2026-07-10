# @ Warm agent runtime

## + Requirement: Oneshot call budget and recovery

Each oneshot work item on a handle — the ensure-hot plus prompt work for one title-summary
or reply-suggestion call — SHALL complete within ten seconds of wall-clock time or SHALL
fail with an error returned to the caller. An oneshot call SHALL NOT remain in flight
indefinitely past that budget. After any oneshot failure, including a timeout that exceeds
the budget, the oneshot path for that handle SHALL cold-reset process heat before serving
further oneshot work. A later oneshot request on the same handle after a failed or
timed-out oneshot SHALL still be able to complete, subject to its own budget. Title
summary and reply suggestion each receive a full ten-second budget per call; they still
run one at a time on the shared oneshot path.

> test: code

### Scenario: Over-budget oneshot returns an error

- **GIVEN** a chat agent handle
- **AND** oneshot work that does not finish within ten seconds
- **WHEN** that oneshot call is awaited
- **THEN** the caller receives an error
- **AND** the call does not remain in flight indefinitely past the budget

> test: code

### Scenario: Later oneshot succeeds after prior oneshot failure

- **GIVEN** a chat agent handle
- **AND** an oneshot call that failed or timed out
- **WHEN** a subsequent oneshot is requested on the same handle
- **THEN** that subsequent call can complete with a result

> test: code
