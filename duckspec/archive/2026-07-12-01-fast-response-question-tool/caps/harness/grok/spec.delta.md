# @ Grok harness

## + Requirement: Structured questions enabled

The main-path Grok agent launch SHALL NOT pass `--no-ask-user`, so the agent may issue
structured user questions. The same launch SHALL still auto-approve tool execution for the
turn (always-approve style), so ordinary tool permission prompts do not require host UI.

> test: code

### Scenario: Main launch does not pass no-ask-user

- **GIVEN** the Grok main-path agent launch
- **WHEN** the launch arguments are inspected
- **THEN** they do not include `--no-ask-user`

### Scenario: Main launch still auto-approves tool execution

- **GIVEN** the Grok main-path agent launch
- **WHEN** the launch arguments are inspected
- **THEN** they include the always-approve flag that auto-approves tool execution

## + Requirement: Question wire mapping

When the Grok agent issues a mid-turn `x.ai/ask_user_question` request, the harness path
SHALL expose that request to the host as a neutral user choice (via the shared ACP client
main path). A host selection SHALL complete the request with an accepted questionnaire
response carrying the chosen answers. A host custom freeform answer SHALL complete the
request with an accepted questionnaire response carrying that freeform text as the answer
value for the question (not skip-interview). A host cancel SHALL complete the request with
a skip-interview response.

> test: code

### Scenario: An ask-user extension request is exposed as a host user choice

- **GIVEN** an in-flight Grok main-path turn
- **AND** an agent `x.ai/ask_user_question` request with at least one option
- **WHEN** the request is handled on the main path
- **THEN** a host user-choice event is emitted for that request

### Scenario: A host selection completes with an accepted questionnaire response

- **GIVEN** a pending Grok ask-user request exposed as a host user choice
- **WHEN** the host answers with a selected option
- **THEN** the agent request is completed with an accepted questionnaire response
- **AND** that response carries the chosen answer for the question

### Scenario: Host custom freeform answer completes with an accepted free-text answer

- **GIVEN** a pending Grok ask-user request exposed as a host user choice

- **AND** a question text from that request

- **WHEN** the host answers with custom freeform text

- **THEN** the agent request is completed with an accepted questionnaire response

- **AND** that response carries an answers entry mapping that question text to that
  freeform text

- **AND** the response is not a skip-interview outcome

### Scenario: A host cancel completes with a skip-interview response

- **GIVEN** a pending Grok ask-user request exposed as a host user choice
- **WHEN** the host answers as cancelled
- **THEN** the agent request is completed with a skip-interview response
