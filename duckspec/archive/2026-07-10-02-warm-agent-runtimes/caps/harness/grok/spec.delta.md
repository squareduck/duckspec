# @ Grok harness

## @ Requirement: Session lifecycle and resume

Running a turn without a prior session id SHALL open a new grok session and report the id
grok assigns. Running a turn with a prior session id SHALL open by resuming that id. In
both cases the reported session id SHALL be surfaced for the caller to persist. When the
main path is already process-hot, a subsequent turn SHALL reuse that process rather than
spawning a new `grok agent stdio` child. Cancelling an in-flight turn SHALL kill the main
child; a later turn SHALL be allowed to spawn again and, when a prior session id is
supplied, resume that id.

> test: code

### + Scenario: A second turn on a hot path reuses the process

- **GIVEN** a completed turn that left the main path process-hot

- **WHEN** a second turn is run on the same main path

- **THEN** the harness does not spawn a new `grok agent stdio` process for that turn

- **AND** the turn still opens or resumes the conversation session as required by the
  session id

> test: code

### + Scenario: After cancel, the next turn can spawn and resume

- **GIVEN** a main path whose in-flight turn was cancelled (main child killed)
- **AND** a prior conversation session id
- **WHEN** a later turn is run with that session id
- **THEN** the harness may spawn a new process
- **AND** it opens the session by resuming that id

> test: code

## + Requirement: Warm oneshot path

Title summary and reply-suggestion calls on the grok oneshot path SHALL reuse a warm
oneshot process when the path is already process-hot, rather than spawning a new
`grok agent stdio` child for each call. Each oneshot call SHALL use a fresh grok ACP
session (N=1) and SHALL NOT resume a prior oneshot conversation session.

> test: code

### Scenario: A second oneshot call does not resume the prior oneshot session

- **GIVEN** a grok oneshot path that has completed one oneshot call
- **WHEN** a second oneshot call is made on that path
- **THEN** the second call opens a fresh grok session
- **AND** it does not resume the prior oneshot session id

> test: code

### Scenario: An oneshot call on a hot path reuses the process

- **GIVEN** a grok oneshot path that is already process-hot
- **WHEN** an oneshot call is made on that path
- **THEN** the harness does not spawn a new `grok agent stdio` process for that call

> test: code
