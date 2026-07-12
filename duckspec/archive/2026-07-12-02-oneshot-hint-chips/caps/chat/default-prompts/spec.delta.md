# @ Chat default prompts

Empty-composer next actions from lifecycle bootstrap or a trailing `next` meta card, shown
as ghost text with empty Enter and Tab cycle; optional settings-gated oneshot reply
suggestions (up to three freeform `REPLY:` lines) that may fill fast-response chips only
when there is no next-action ghost.

## - Requirement: Oneshot empty-input send

## - Requirement: Oneshot presentation

## @ Requirement: Parsed suggestion list

Raw model output SHALL be reduced to at most three non-empty strings taken from lines that
start with the prefix `REPLY:`, in source order. Lines that do not match that prefix SHALL
be ignored. Text after the prefix is trimmed; empty results after trim are dropped.
Unknown slash forms (including command names not in any allow-list) SHALL be kept as
written. A soft character budget in the oneshot instruction SHALL NOT cause the parser to
truncate reply text — over-budget strings SHALL be kept in full after trim. When more than
three `REPLY:` lines are present, only the first three non-empty results SHALL be kept.
When fewer than three non-empty results are present, all of them SHALL be kept in order.

### - Scenario: REPLY lines capped at one

### + Scenario: REPLY lines capped at three

- **GIVEN** model output with four lines each starting with `REPLY:` and non-empty text
- **WHEN** the suggestion list is parsed
- **THEN** the list has exactly three entries
- **AND** those entries are the first three reply texts in source order

> test: code

### + Scenario: Fewer than three REPLY lines are kept as-is

- **GIVEN** model output with two lines each starting with `REPLY:` and non-empty text
- **WHEN** the suggestion list is parsed
- **THEN** the list has exactly two entries
- **AND** those entries are the two reply texts in source order

> test: code

## @ Requirement: Oneshot request framing

The reply-suggestion request SHALL carry the full last assistant message and the preceding
user message when present, without line-count truncation and without a truncation marker
for omitted earlier lines. The request SHALL NOT include a lifecycle heuristic. The
request SHALL NOT include discovered slash command names as priming hints. The instruction
framing SHALL ask for up to three lines of the form `REPLY: <text>` that suggest natural
freeform user responses continuing the dialogue — in order: most likely reply, alternative
reply, and negative or decline reply — and SHALL allow omitting a line when it does not
fit. The instruction SHALL NOT prefer duckspec stage slash commands as the default job of
the oneshot. An empty assistant message SHALL yield an empty suggestion list without
calling the model.

### - Scenario: Instruction asks for a freeform user reply and at most one REPLY line

### + Scenario: Instruction asks for up to three ordered freeform REPLY lines

- **GIVEN** the shared reply-suggestion instruction text

- **WHEN** the instruction is inspected

- **THEN** it asks for natural freeform user replies continuing the dialogue

- **AND** it allows up to three `REPLY:` lines in order most likely, alternative, and
  negative or decline

- **AND** it does not prefer stage slash commands as the oneshot's primary job

> test: code

## @ Requirement: Oneshot readiness

After a non-priming turn completes and a reply-suggestion oneshot is started for a
generation, oneshot suggestions SHALL be pending until that oneshot settles (success or
failure for any reason, including oneshot timeout) for the same generation. While pending,
the system SHALL NOT present oneshot loading chrome and SHALL NOT present oneshot chip
options from that in-flight generation. Pending oneshot state SHALL NOT block empty Enter
from sending an armed next action. When the oneshot settles for the matching generation,
oneshot suggestions SHALL become ready: a non-empty parse is the settled list used for
chip eligibility; an empty parse or failure yields a ready empty list. Results for a
superseded generation SHALL NOT change the session's ready oneshot list. When no oneshot
is outstanding, oneshot suggestions are ready (the list may be empty). If the chat agent
handle ends while a reply-suggestion oneshot is outstanding for the current generation
without a matching settle, oneshot suggestions SHALL become ready (they SHALL NOT remain
pending). There is no under-input oneshot suggestion row and no empty Cmd-Enter path for
oneshot suggestions.

### - Scenario: Pending hides oneshot row and shows loading

### - Scenario: Empty Cmd-Enter is a no-op while oneshot pending

### - Scenario: Ready after settle arms the oneshot row

### - Scenario: Main turn in progress hides oneshot chrome

### - Scenario: Timed-out or failed oneshot settles to ready empty

### - Scenario: Agent handle ends while oneshot pending becomes ready

### + Scenario: Failed or timed-out oneshot settles without presenting suggestions

- **GIVEN** a pending reply-suggestion oneshot
- **AND** that oneshot settles as a failure for the current generation
- **WHEN** the settled oneshot list is inspected
- **THEN** oneshot suggestions are ready
- **AND** the settled list is empty when the failure produced no parse

> test: code

### + Scenario: Agent handle end while pending leaves suggestions ready empty

- **GIVEN** a pending reply-suggestion oneshot
- **AND** the chat agent handle ends without a settle for that generation
- **WHEN** oneshot readiness is inspected
- **THEN** oneshot suggestions are ready
- **AND** no oneshot loading chrome is shown

> test: code

### + Scenario: Pending oneshot presents no loading chrome

- **GIVEN** a pending reply-suggestion oneshot
- **AND** an empty composer input
- **WHEN** empty-composer oneshot chrome is evaluated
- **THEN** no oneshot loading indicator is shown
- **AND** no oneshot suggestion row is shown under the input

> test: code

## @ Requirement: Agent input hints gate

A global agent input hints setting SHALL control whether reply-suggestion oneshots run
after turns. The setting SHALL default to disabled. When agent input hints is disabled, a
reply-suggestion oneshot SHALL NOT be started after a non-priming turn completes. When
agent input hints is enabled, oneshot launch follows the non-priming turn rules of this
capability (assistant text present and other launch conditions) and SHALL NOT start when
the next-action list for that session is non-empty after the turn's next-action refresh.
There is no separate auto-messages setting that suppresses oneshots or next-action lists.
Empty-session next-action bootstrap and the next-action list SHALL NOT depend on the agent
input hints setting.

### + Scenario: Oneshot launch is skipped when the next-action list is non-empty

- **GIVEN** agent input hints enabled
- **AND** a non-priming turn that has assistant text
- **AND** a non-empty next-action list after that turn's next-action refresh
- **WHEN** oneshot launch is decided
- **THEN** a reply-suggestion oneshot is not started

> test: code

## + Requirement: Oneshot chip eligibility

Settled oneshot replies SHALL be eligible to fill fast-response chips only when all of the
following hold: agent input hints is enabled; no main agent turn is streaming; the session
is not awaiting a user choice; the next-action list is empty; and the settled oneshot list
is non-empty. When any of those conditions fails, oneshot replies SHALL NOT be eligible to
fill chips.

> test: code

### Scenario: Eligible when idle with no next actions and a settled list

- **GIVEN** agent input hints enabled
- **AND** no main agent turn in progress
- **AND** the session is not awaiting a user choice
- **AND** an empty next-action list
- **AND** a non-empty settled oneshot list
- **WHEN** oneshot chip eligibility is evaluated
- **THEN** oneshot replies are eligible to fill chips

> test: code

### Scenario: Ineligible when next-action list is non-empty

- **GIVEN** agent input hints enabled
- **AND** no main agent turn in progress
- **AND** the session is not awaiting a user choice
- **AND** a non-empty next-action list
- **AND** a non-empty settled oneshot list
- **WHEN** oneshot chip eligibility is evaluated
- **THEN** oneshot replies are not eligible to fill chips

> test: code

### Scenario: Ineligible while awaiting a user choice

- **GIVEN** agent input hints enabled
- **AND** the session is awaiting a user choice
- **AND** an empty next-action list
- **AND** a non-empty settled oneshot list
- **WHEN** oneshot chip eligibility is evaluated
- **THEN** oneshot replies are not eligible to fill chips

> test: code

### Scenario: Ineligible while streaming

- **GIVEN** agent input hints enabled
- **AND** a main agent turn in progress
- **AND** the session is not awaiting a user choice
- **AND** an empty next-action list
- **AND** a non-empty settled oneshot list
- **WHEN** oneshot chip eligibility is evaluated
- **THEN** oneshot replies are not eligible to fill chips

> test: code

### Scenario: Ineligible when the settled list is empty

- **GIVEN** agent input hints enabled
- **AND** no main agent turn in progress
- **AND** the session is not awaiting a user choice
- **AND** an empty next-action list
- **AND** an empty settled oneshot list
- **WHEN** oneshot chip eligibility is evaluated
- **THEN** oneshot replies are not eligible to fill chips

> test: code
