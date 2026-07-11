# @ ACP client

## + Requirement: Mid-turn tool permission auto-allow

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

## + Requirement: Mid-turn user choice

When an agent issues a mid-turn structured question request (including Grok's
`x.ai/ask_user_question`, or a permission-shaped request that presents product choice
options rather than only allow/reject kinds), the main-path client SHALL surface a neutral
user-choice event to the host with the request's options and SHALL park until the host
answers or the turn is cancelled. Completing with a selected option SHALL write the
protocol-correct success result for that request. Completing with a custom freeform answer
SHALL write the protocol-correct success result that carries that freeform text as the
answer payload for the question. Completing as cancelled SHALL write the protocol-correct
cancelled outcome for that request.

> test: code

### Scenario: Structured question request surfaces a user-choice event

- **GIVEN** an in-flight main-path turn
- **AND** an agent structured question request with at least one option
- **WHEN** the client handles that request
- **THEN** a user-choice event is emitted carrying those options
- **AND** the agent request remains open until answered or cancelled

### Scenario: Host selected answer completes the pending request

- **GIVEN** a pending user-choice event on the main path
- **WHEN** the host answers with a selected option id
- **THEN** the agent request is completed successfully for that selection
- **AND** the turn may continue after the completion

### Scenario: Host custom freeform answer completes the pending request

- **GIVEN** a pending user-choice event on the main path

- **WHEN** the host answers with custom freeform text

- **THEN** the agent request is completed successfully with that freeform text as the
  answer payload

- **AND** the request is not completed as cancelled

### Scenario: Host cancel completes the pending request as cancelled

- **GIVEN** a pending user-choice event on the main path
- **WHEN** the host answers as cancelled
- **THEN** the agent request is completed as cancelled

### Scenario: Turn cancel completes a pending choice as cancelled

- **GIVEN** a pending user-choice event on the main path
- **WHEN** the in-flight turn is cancelled
- **THEN** the agent request is completed as cancelled

## + Requirement: Headless and oneshot safety

On the oneshot path, the client SHALL NOT block a call waiting for a host UI choice. Agent
requests that would require a structured host choice on the main path SHALL be completed
without parking on oneshot so headless oneshot work cannot deadlock.

> test: code

### Scenario: Oneshot path does not block waiting on a host UI choice

- **GIVEN** an oneshot-path call
- **AND** an agent request that would surface as a user choice on the main path
- **WHEN** the client handles that request on oneshot
- **THEN** the call completes without waiting for a host UI answer
