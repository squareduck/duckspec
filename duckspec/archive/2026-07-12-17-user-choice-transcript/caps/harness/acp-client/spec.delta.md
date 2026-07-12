# @ ACP client

## @ Requirement: Mid-turn user choice

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

### ~ Scenario: Structured question request surfaces a user-choice event

- **GIVEN** an in-flight main-path turn

- **AND** an agent structured question request with at least one option and non-empty
  question text

- **WHEN** the client handles that request

- **THEN** a user-choice event is emitted carrying those options

- **AND** the user-choice event carries that question text as the prompt

- **AND** the agent request remains open until answered or cancelled

> test: code

### + Scenario: Permission product choice carries prompt from tool title

- **GIVEN** an in-flight main-path turn

- **AND** an agent `session/request_permission` whose options are product choices (not
  only allow/reject kinds)

- **AND** the request includes a non-empty tool-call title

- **WHEN** the client classifies that request as a user choice

- **THEN** the user-choice event carries that title as the prompt

- **AND** the user-choice event carries the product options

> test: code
