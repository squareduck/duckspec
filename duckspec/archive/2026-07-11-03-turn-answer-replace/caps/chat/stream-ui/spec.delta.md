# @ Chat stream UI

## @ Requirement: Bounded materialization while streaming

While a turn is streaming, pure answer or reasoning content deltas alone SHALL NOT each
force chat UI materialization. Accumulated pure-content dirtiness SHALL materialize on the
stream UI tick only while the transcript is stick-to-bottom (the user is following the
live answer). While the user has scrolled up to read history, pure-content dirtiness SHALL
remain deferred so the chat column is not rebuilt under their scroll; re-engaging
stick-to-bottom SHALL materialize any deferred pure content. Structural transcript changes
— tool use, tool result, a kind switch between answer and reasoning channels (whether or
not the open answer draft is committed), turn complete, error, or process exit — SHALL
materialize the chat UI as part of handling that event, without waiting for a stream UI
tick and regardless of stick-to-bottom.

> test: code

### + Scenario: Answer-to-reasoning channel switch materializes without committing the answer

- **GIVEN** a streaming turn with a non-empty live answer draft
- **WHEN** a reasoning content delta is applied (answer channel to reasoning channel)
- **THEN** chat UI materialization runs as part of handling that event
- **AND** the open answer draft remains uncommitted on the session

> test: code

## + Requirement: Answer draft across thought

While a turn is streaming, a reasoning content delta SHALL NOT commit the open answer
draft into the session’s messages. When answer content resumes after reasoning while an
answer draft is already open, the session SHALL replace that draft with the new answer
content (the prior draft text is discarded). Applying a tool use SHALL commit the open
answer draft into the session’s messages before the tool is recorded.

> test: code

### Scenario: Reasoning leaves the open answer uncommitted

- **GIVEN** a streaming turn with a non-empty live answer draft

- **WHEN** a reasoning content delta is applied

- **THEN** the session still holds that answer text only as the live answer draft

- **AND** the session’s committed messages do not gain a new answer text block for that
  draft

> test: code

### Scenario: Answer after reasoning replaces the live draft

- **GIVEN** a streaming turn whose live answer draft is a known first body
- **AND** a reasoning content delta has been applied after that draft
- **WHEN** an answer content delta with a different second body is applied
- **THEN** the live answer draft is the second body
- **AND** the live answer draft does not retain the first body

> test: code

### Scenario: Tool use commits the open answer draft

- **GIVEN** a streaming turn with a non-empty live answer draft
- **WHEN** a tool use is applied to the session
- **THEN** the session’s committed messages include that answer text
- **AND** the live answer draft is empty

> test: code

## + Requirement: Answer thrash budget

Within one streaming turn, after two answer-after-thought draft replacements, a third
answer-after-thought replacement SHALL cancel the in-flight turn, keep the last live
answer draft as the turn’s answer, and surface a short stop notice that is not an answer
rewrite. The replacement count SHALL reset when a tool use is applied so answer spans
separated by tools do not share a budget.

> test: code

### Scenario: Third answer-after-thought cancels and keeps the last draft

- **GIVEN** a streaming turn that has already replaced the live answer draft twice after
  reasoning (two answer-after-thought replacements)

- **WHEN** a third answer-after-thought replacement begins (answer content after reasoning
  with a non-empty draft)

- **THEN** the in-flight turn is cancelled

- **AND** the session keeps the last live answer draft as the turn’s answer

- **AND** a short stop notice is shown that is not a second full answer rewrite

> test: code

### Scenario: Tool use resets the thrash budget

- **GIVEN** a streaming turn that has already performed two answer-after-thought draft
  replacements

- **AND** a tool use has since been applied (budget reset)

- **WHEN** answer content is applied after further reasoning with a non-empty draft
  (another answer-after-thought replacement)

- **THEN** the in-flight turn is not cancelled solely for exceeding the thrash budget

> test: code
