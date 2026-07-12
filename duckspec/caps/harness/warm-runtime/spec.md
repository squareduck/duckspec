# Warm agent runtime

Per-chat agent handle owns a main path and a single oneshot path. Main and oneshot
activate lazily on first send; titles and reply suggestions go through the handle,
serialize on the oneshot path, and isolate each oneshot call (N=1). Cancel ends main
process heat; the next send may re-warm.

## Requirement: Per-chat handle ownership

Each chat agent worker SHALL expose one main path and one oneshot path bound to that
chat’s handle only — not an app-global pool. Title summaries and reply suggestions for
that chat SHALL be requested through the same handle that runs its turns. Distinct chat
handles SHALL NOT share oneshot work with each other.

> test: code

### Scenario: Title summary is requested through the chat handle

- **GIVEN** a chat agent handle for a session
- **WHEN** a title summary is requested for that session
- **THEN** the request is served through that handle
- **AND** the result is a plain-text title string

> test: code
> - crates/duckchat/src/worker.rs:681

### Scenario: Reply suggestions are requested through the chat handle

- **GIVEN** a chat agent handle for a session
- **WHEN** reply suggestions are requested for that session
- **THEN** the request is served through that handle
- **AND** the result is a list of reply strings

> test: code
> - crates/duckchat/src/worker.rs:702

## Requirement: Lazy activation

Main and oneshot paths SHALL NOT be required to be active before the chat’s first turn.
The first turn on a handle SHALL activate the main path as needed. Title summary and reply
suggestions SHALL be usable after the first send without a separate caller-facing pre-warm
step.

> test: code

### Scenario: First turn succeeds without a prior pre-warm call

- **GIVEN** a newly ready chat agent handle that has not run a turn
- **WHEN** the first turn is run on that handle
- **THEN** the turn completes without requiring a separate pre-warm call

> test: code
> - crates/duckchat/src/worker.rs:723

### Scenario: Oneshot after first send needs no separate pre-warm API

- **GIVEN** a chat agent handle that has accepted its first send
- **WHEN** a title summary or reply-suggestion request is made on that handle
- **THEN** the request completes without a separate pre-warm call from the caller

> test: code
> - crates/duckchat/src/worker.rs:745

## Requirement: Oneshot serialization and isolation

Title summary and reply suggestions SHALL share one oneshot path per handle and SHALL run
one at a time on that path. Each oneshot call SHALL use a fresh logical session (N=1): it
SHALL NOT resume a prior oneshot conversation. After an oneshot result is returned to the
caller, the next oneshot call on that handle SHALL again be isolated from prior oneshot
history.

> test: code

### Scenario: Title and reply suggestions run one at a time on the oneshot path

- **GIVEN** a chat agent handle
- **AND** both a title-summary request and a reply-suggestion request are outstanding
- **WHEN** both requests complete
- **THEN** each request receives its own result
- **AND** the oneshot path did not run the two prompts concurrently

> test: code
> - crates/duckchat/src/worker.rs:764

### Scenario: A second oneshot call does not resume the prior oneshot session

- **GIVEN** a chat agent handle that has completed one oneshot call
- **WHEN** a second oneshot call is made on that handle
- **THEN** the second call does not resume the prior oneshot conversation

> test: code
> - crates/duckchat/src/worker.rs:796

## Requirement: Cancel and re-warm

Cancelling an in-flight main turn SHALL end main process heat for that handle. A
subsequent turn on the same handle SHALL still be able to run (re-warm is allowed). Cancel
SHALL NOT be required to tear down the oneshot path.

> test: code

### Scenario: After cancel, a later turn on the same handle can complete

- **GIVEN** a chat agent handle whose in-flight main turn was cancelled
- **WHEN** a later turn is run on that handle
- **THEN** the later turn can complete

> test: code
> - crates/duckchat/src/worker.rs:828

## Requirement: Cold-capable harnesses

A harness that cannot keep a process warm SHALL still satisfy this capability by
performing equivalent work per call (no-op heat). Callers SHALL use the same handle API
for title summary and reply suggestions regardless of whether the harness reuses a
process.

> test: code

### Scenario: A cold-capable harness serves title summary through the handle

- **GIVEN** a chat agent handle backed by a harness that does not reuse a process
- **WHEN** a title summary is requested through that handle
- **THEN** the request completes with a plain-text title string

> test: code
> - crates/duckchat/src/worker.rs:870

## Requirement: Oneshot call budget and recovery

Each oneshot work item on a handle — the ensure-hot plus prompt work for one title-summary
or reply-suggestion call — SHALL complete within the **oneshot call budget** or SHALL fail
with an error returned to the caller. The oneshot call budget is **thirty seconds** of
wall-clock time. An oneshot call SHALL NOT remain in flight indefinitely past that budget.
After any oneshot failure, including a timeout that exceeds the budget, the oneshot path
for that handle SHALL cold-reset process heat before serving further oneshot work. A later
oneshot request on the same handle after a failed or timed-out oneshot SHALL still be able
to complete, subject to its own budget. Title summary and reply suggestion each receive a
full oneshot call budget per call; they still run one at a time on the shared oneshot
path.

### Scenario: Over-budget oneshot returns an error

- **GIVEN** a chat agent handle
- **AND** oneshot work that does not finish within the oneshot call budget
- **WHEN** that oneshot call is awaited
- **THEN** the caller receives an error
- **AND** the call does not remain in flight indefinitely past the budget

> test: code
> - crates/duckchat/src/worker.rs:893

### Scenario: Later oneshot succeeds after prior oneshot failure

- **GIVEN** a chat agent handle
- **AND** an oneshot call that failed or timed out
- **WHEN** a subsequent oneshot is requested on the same handle
- **THEN** that subsequent call can complete with a result

> test: code
> - crates/duckchat/src/worker.rs:950
