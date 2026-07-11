# ACP client

The shared ACP client runtime drives every harness turn: it spawns a launch-parameterized
agent child, opens or resumes a session, maps profile `session/update` notifications into
neutral agent events, and keeps the main agent process warm across turns until cancel.

## Requirement: Launch-parameterized agent process

The client SHALL spawn the agent process defined by the harness launch (program and
arguments the provider supplies), not a hardcoded binary. When the main path is already
process-hot, a subsequent turn SHALL reuse that agent process rather than spawning a new
child. Cancelling an in-flight turn SHALL kill the main agent child; a later turn SHALL be
allowed to spawn again and, when a prior session id is supplied, resume that id.

> test: code

### Scenario: The client spawns the launch-supplied agent command

- **GIVEN** a harness launch that names a specific agent command
- **WHEN** the client opens a main path and runs a turn
- **THEN** the spawned agent process is that launch-supplied command

> test: code
> - crates/duckchat/src/acp/turn.rs:573

### Scenario: A second turn on a hot main path reuses the agent process

- **GIVEN** a completed turn that left the main path process-hot

- **WHEN** a second turn is run on the same main path

- **THEN** the client does not spawn a new agent process for that turn

- **AND** the turn still opens or resumes the conversation session as required by the
  session id

> test: code
> - crates/duckchat/src/acp/runtime.rs:731

### Scenario: After cancel, a later turn may spawn again and resume a prior session id

- **GIVEN** a main path whose in-flight turn was cancelled (main agent child killed)
- **AND** a prior conversation session id
- **WHEN** a later turn is run with that session id
- **THEN** the client may spawn a new agent process
- **AND** it opens the session by resuming that id

> test: code
> - crates/duckchat/src/acp/runtime.rs:766

## Requirement: Session open and resume

Running a turn without a prior session id SHALL open a new agent session and report the id
the agent assigns. Running a turn with a prior session id SHALL open by resuming that id.
In both cases the reported session id SHALL be surfaced for the caller to persist. When
the agent rebinds the session id during a turn (assigns a different id than the one
returned at open), the client SHALL surface that rebound id for the caller to persist.
When resuming a session id the agent reports as missing, the client SHALL surface a
session-not-found outcome so the caller can drop the id and retry.

> test: code

### Scenario: A turn without a prior session id opens a new session and surfaces the id

- **GIVEN** a turn request carrying no session id
- **WHEN** the client runs the turn
- **THEN** it opens a fresh agent session
- **AND** it surfaces the session id the agent assigned

> test: code
> - crates/duckchat/src/acp/turn.rs:485

### Scenario: A turn with a prior session id resumes that id

- **GIVEN** a turn request carrying a previously assigned session id
- **WHEN** the client runs the turn
- **THEN** it opens the session by resuming that same id

> test: code
> - crates/duckchat/src/acp/turn.rs:525

### Scenario: When the agent rebinds the session id during a turn, the client surfaces the rebound id

- **GIVEN** a turn that opened under one session id
- **AND** the agent rebinds that turn to a different session id before the turn completes
- **WHEN** the client finishes the turn
- **THEN** it surfaces the rebound session id for the caller to persist

> test: code
> - crates/duckchat/src/acp/runtime.rs:691

### Scenario: A failed load of a missing session surfaces session-not-found

- **GIVEN** a turn request carrying a session id the agent cannot load
- **WHEN** the client opens the session
- **THEN** the outcome is session-not-found rather than a successful resume

> test: code
> - crates/duckchat/src/acp/turn.rs:544

## Requirement: Profile event translation

The client SHALL translate profile `session/update` notifications into neutral agent
events: assistant text and reasoning SHALL surface on separate channels; a tool invocation
SHALL surface as a tool-use event followed by a result event sharing the same call id; and
token telemetry SHALL surface as a usage update carrying the used-token count together
with the active model's context window when known.

> test: code

### Scenario: Assistant text and reasoning surface on distinct channels

- **GIVEN** a session update stream containing both an assistant message chunk and a
  reasoning chunk

- **WHEN** the client translates the stream

- **THEN** the assistant text is emitted as a content event

- **AND** the reasoning text is emitted as a separate reasoning event

> test: code
> - crates/duckchat/src/acp/event.rs:123

### Scenario: A tool call surfaces as a use then a matching result

- **GIVEN** a session update stream containing a tool call and its completion
- **WHEN** the client translates the stream
- **THEN** a tool-use event is emitted with the call's id, name, and input
- **AND** a tool-result event is emitted carrying the same call id and the tool output

> test: code
> - crates/duckchat/src/acp/event.rs:153

### Scenario: A usage update carries used tokens and the model's context window

- **GIVEN** a session update reporting a running total-token count
- **AND** an active model whose context window is known
- **WHEN** the client translates the update
- **THEN** a usage event is emitted with that used-token count and that context window

> test: code
> - crates/duckchat/src/acp/event.rs:196
