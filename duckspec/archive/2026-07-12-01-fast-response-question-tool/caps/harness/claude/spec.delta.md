# @ Claude harness

## + Requirement: AskUserQuestion available

The owned Claude ACP agent SHALL NOT list `AskUserQuestion` among tools disallowed for the
official `claude` backend, so Claude may issue structured clarifying questions during a
turn.

> test: code

### Scenario: AskUserQuestion is not among disallowed tools

- **GIVEN** the owned Claude ACP agent's backend launch configuration
- **WHEN** the disallowed-tools list is inspected
- **THEN** it does not include `AskUserQuestion`

## + Requirement: Mid-prompt parent choice

When Claude issues an `AskUserQuestion` request during a turn (via the stream-json control
/ canUseTool path), the owned agent SHALL surface a structured choice to the ACP parent so
the host receives a neutral user-choice event. Completing that choice with a selection
SHALL finish Claude's request as allow with an answers map from question text to selected
option label. Completing with a custom freeform answer SHALL finish Claude's request as
allow with an answers map from question text to that freeform text (not deny). Completing
as cancelled SHALL finish Claude's request without accepting the questionnaire (deny or
equivalent skip).

> test: code

### Scenario: An AskUserQuestion request surfaces a host user choice

- **GIVEN** an in-flight Claude main-path turn
- **AND** Claude issuing an AskUserQuestion with at least one option
- **WHEN** the owned agent handles that request
- **THEN** the ACP parent surfaces a host user-choice event for those options

### Scenario: Host selection completes with allow and answers

- **GIVEN** a pending AskUserQuestion exposed as a host user choice

- **WHEN** the host answers with a selected option label for the question

- **THEN** Claude's request is completed as allow

- **AND** the updated input includes an answers entry mapping that question text to that
  label

### Scenario: Host custom freeform answer completes with allow and free-text answers

- **GIVEN** a pending AskUserQuestion exposed as a host user choice

- **AND** a question text from that request

- **WHEN** the host answers with custom freeform text

- **THEN** Claude's request is completed as allow

- **AND** the updated input includes an answers entry mapping that question text to that
  freeform text

- **AND** the request is not completed as deny

### Scenario: Host cancel completes without accepting the questionnaire

- **GIVEN** a pending AskUserQuestion exposed as a host user choice
- **WHEN** the host answers as cancelled
- **THEN** Claude's request is completed without accepting the questionnaire

## + Requirement: Ordinary tools stay auto-approved

Non-question tool invocations on the Claude main path SHALL NOT require host UI when the
backend is configured for permission bypass of ordinary tools. AskUserQuestion remains the
structured-choice path for clarifying questions.

> test: code

### Scenario: Non-question tools do not require host UI under bypass

- **GIVEN** a Claude main-path turn with ordinary-tool permission bypass enabled
- **AND** Claude invoking a non-question tool that is not AskUserQuestion
- **WHEN** the owned agent handles that tool permission
- **THEN** the tool is allowed without emitting a host user-choice event
