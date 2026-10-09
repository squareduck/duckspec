# ACP client

The shared ACP client runtime drives every harness turn: it spawns a launch-parameterized
agent child, opens or resumes a session, maps profile `session/update` notifications into
neutral agent events, and keeps the main agent process warm across turns until cancel.

The shared ACP client runtime drives every harness turn: it spawns a launch-parameterized
agent child, opens or resumes a session, maps profile `session/update` notifications into
neutral agent events, and keeps the main agent process warm across turns until cancel.
Initialize may carry optional per-model effort and catalog status, and a turn's effort
level is copied onto `session/prompt` as `effort`.

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
> - crates/duckchat/src/acp/turn.rs:1198

### Scenario: A second turn on a hot main path reuses the agent process

- **GIVEN** a completed turn that left the main path process-hot

- **WHEN** a second turn is run on the same main path

- **THEN** the client does not spawn a new agent process for that turn

- **AND** the turn still opens or resumes the conversation session as required by the
  session id

> test: code
> - crates/duckchat/src/acp/runtime.rs:914

### Scenario: After cancel, a later turn may spawn again and resume a prior session id

- **GIVEN** a main path whose in-flight turn was cancelled (main agent child killed)
- **AND** a prior conversation session id
- **WHEN** a later turn is run with that session id
- **THEN** the client may spawn a new agent process
- **AND** it opens the session by resuming that id

> test: code
> - crates/duckchat/src/acp/runtime.rs:955

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
> - crates/duckchat/src/acp/turn.rs:955

### Scenario: A turn with a prior session id resumes that id

- **GIVEN** a turn request carrying a previously assigned session id
- **WHEN** the client runs the turn
- **THEN** it opens the session by resuming that same id

> test: code
> - crates/duckchat/src/acp/turn.rs:1151

### Scenario: When the agent rebinds the session id during a turn, the client surfaces the rebound id

- **GIVEN** a turn that opened under one session id
- **AND** the agent rebinds that turn to a different session id before the turn completes
- **WHEN** the client finishes the turn
- **THEN** it surfaces the rebound session id for the caller to persist

> test: code
> - crates/duckchat/src/acp/runtime.rs:869

### Scenario: A failed load of a missing session surfaces session-not-found

- **GIVEN** a turn request carrying a session id the agent cannot load
- **WHEN** the client opens the session
- **THEN** the outcome is session-not-found rather than a successful resume

> test: code
> - crates/duckchat/src/acp/turn.rs:1169

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
> - crates/duckchat/src/acp/event.rs:122

### Scenario: A tool call surfaces as a use then a matching result

- **GIVEN** a session update stream containing a tool call and its completion
- **WHEN** the client translates the stream
- **THEN** a tool-use event is emitted with the call's id, name, and input
- **AND** a tool-result event is emitted carrying the same call id and the tool output

> test: code
> - crates/duckchat/src/acp/event.rs:152

### Scenario: A usage update carries used tokens and the model's context window

- **GIVEN** a session update reporting a running total-token count
- **AND** an active model whose context window is known
- **WHEN** the client translates the update
- **THEN** a usage event is emitted with that used-token count and that context window

> test: code
> - crates/duckchat/src/acp/event.rs:195

## Requirement: Mid-turn tool permission auto-allow

When an agent issues a mid-turn `session/request_permission` whose options are only
permission kinds (allow/reject once or always), the main-path client SHALL complete that
request by selecting an allow option without waiting on host UI. The turn SHALL continue
after the auto-allow.

> test: code

### Scenario: Permission request with only allow/reject kinds is auto-allowed

- **GIVEN** an in-flight main-path turn
- **AND** an agent `session/request_permission` whose options are only allow/reject kinds
- **WHEN** the client handles that request
- **THEN** the request is completed with an allow selection
- **AND** the client does not emit a host user-choice event for that request

> test: code
> - crates/duckchat/src/acp/turn.rs:1370

## Requirement: Mid-turn user choice

When an agent issues a mid-turn structured question request (including Grok's
`x.ai/ask_user_question`, or a permission-shaped request that presents product choice
options rather than only allow/reject kinds), the main-path client SHALL surface a neutral
user-choice event to the host with the request's options and SHALL park until the host
answers or the turn is cancelled. When the request supplies question text — including a
Grok questionnaire question field or a non-empty permission `toolCall` title — the
user-choice event SHALL carry that text as the prompt. Completing with a selected option
SHALL write the protocol-correct success result for that request. Completing with a custom
freeform answer SHALL write the protocol-correct success result that carries that freeform
text as the answer payload for the question. Completing as cancelled SHALL write the
protocol-correct cancelled outcome for that request.

> test: code

### Scenario: Structured question request surfaces a user-choice event

- **GIVEN** an in-flight main-path turn

- **AND** an agent structured question request with at least one option and non-empty
  question text

- **WHEN** the client handles that request

- **THEN** a user-choice event is emitted carrying those options

- **AND** the user-choice event carries that question text as the prompt

- **AND** the agent request remains open until answered or cancelled

> test: code
> - crates/duckchat/src/acp/turn.rs:1427

### Scenario: Host selected answer completes the pending request

- **GIVEN** a pending user-choice event on the main path
- **WHEN** the host answers with a selected option id
- **THEN** the agent request is completed successfully for that selection
- **AND** the turn may continue after the completion

> test: code
> - crates/duckchat/src/acp/turn.rs:1506

### Scenario: Host custom freeform answer completes the pending request

- **GIVEN** a pending user-choice event on the main path

- **WHEN** the host answers with custom freeform text

- **THEN** the agent request is completed successfully with that freeform text as the
  answer payload

- **AND** the request is not completed as cancelled

> test: code
> - crates/duckchat/src/acp/turn.rs:1831

### Scenario: Host cancel completes the pending request as cancelled

- **GIVEN** a pending user-choice event on the main path
- **WHEN** the host answers as cancelled
- **THEN** the agent request is completed as cancelled

> test: code
> - crates/duckchat/src/acp/turn.rs:1565

### Scenario: Turn cancel completes a pending choice as cancelled

- **GIVEN** a pending user-choice event on the main path
- **WHEN** the in-flight turn is cancelled
- **THEN** the agent request is completed as cancelled

> test: code
> - crates/duckchat/src/acp/turn.rs:1617

### Scenario: Permission product choice carries prompt from tool title

- **GIVEN** an in-flight main-path turn

- **AND** an agent `session/request_permission` whose options are product choices (not
  only allow/reject kinds)

- **AND** the request includes a non-empty tool-call title

- **WHEN** the client classifies that request as a user choice

- **THEN** the user-choice event carries that title as the prompt

- **AND** the user-choice event carries the product options

> test: code
> - crates/duckchat/src/acp/turn.rs:1772

## Requirement: Headless and oneshot safety

On the oneshot path, the client SHALL NOT block a call waiting for a host UI choice. Agent
requests that would require a structured host choice on the main path SHALL be completed
without parking on oneshot so headless oneshot work cannot deadlock.

> test: code

### Scenario: Oneshot path does not block waiting on a host UI choice

- **GIVEN** an oneshot-path call
- **AND** an agent request that would surface as a user choice on the main path
- **WHEN** the client handles that request on oneshot
- **THEN** the call completes without waiting for a host UI answer

> test: code
> - crates/duckchat/src/acp/turn.rs:1682

## Requirement: Initialize handshake metadata

Optional per-model effort and an optional catalog status ride on initialize. A handshake
that omits them stays a normal model list.

> test: code

### Scenario: Advertised effort is carried when the model sends it

- **GIVEN** an initialize handshake with two models
- **AND** one model includes an effort object with a default and a list of levels
- **AND** the other model includes no effort object
- **WHEN** the client parses the handshake
- **THEN** the first model carries that default and those levels
- **AND** the second model carries no effort

> test: code
> - crates/duckchat/src/acp/turn.rs:1043

### Scenario: Catalog status is optional handshake metadata

- **GIVEN** a handshake with no catalog object
- **AND** a handshake whose catalog object is ok and includes an expiry
- **AND** a handshake whose catalog object is ok and includes no expiry
- **AND** a handshake whose catalog object is failed
- **WHEN** the client parses each handshake
- **THEN** the handshake with no catalog object reads as ok with no expiry
- **AND** the ok object that includes an expiry keeps ok and that expiry
- **AND** the ok object with no expiry keeps ok and no expiry
- **AND** the failed object keeps failed

> test: code
> - crates/duckchat/src/acp/turn.rs:1091

## Requirement: Prompt effort

A turn's effort level is a `session/prompt` field of its own, separate from the
reasoning-mode field.

> test: code

### Scenario: Effort and reasoning mode use different prompt fields

- **GIVEN** a turn request that carries an effort level and no reasoning mode

- **AND** a turn request that carries a reasoning mode and no effort level

- **AND** a turn request that carries neither

- **WHEN** the client builds `session/prompt` for each

- **THEN** the effort-level turn sends that level as `effort` and leaves `reasoningEffort`
  unset

- **AND** the reasoning-mode turn sends `reasoningEffort` and leaves `effort` unset

- **AND** the turn with neither omits both

> test: code
> - crates/duckchat/src/acp/runtime.rs:747
