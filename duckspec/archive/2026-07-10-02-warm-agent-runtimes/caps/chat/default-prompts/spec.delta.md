# @ Chat default prompts

Conversation-local empty-input defaults: the lifecycle heuristic arms the list before any
oneshot and after a failed or empty oneshot; a settled oneshot with parsed replies
replaces the list. Show and arm after readiness rules; drive empty Enter plus Tab cycling
from the effective list.

## = Requirement: Effective list is oneshot result only

Requirement: Effective default-prompt list

## ~ Requirement: Effective default-prompt list

The effective default-prompt list is built as follows. When a reply-suggestion oneshot has
settled for the current generation with one or more parsed reply strings, the effective
list SHALL be exactly those strings (order preserved, already capped); the lifecycle
heuristic SHALL NOT be appended or merged into that list. When no such non-empty oneshot
result is armed — including a brand-new session that has never run a oneshot, and a
settled oneshot that failed or produced no suggestions — and a lifecycle heuristic is
present, the effective list SHALL be a single entry: that heuristic in empty-send form
(leading `/` when the heuristic is a skill name such as `ds-explore`). When neither a
non-empty oneshot result nor a heuristic is available, the effective list SHALL be empty.
Pre-oneshot heuristic defaults SHALL be ready without a model call (no pending oneshot
required).

> test: code

### Scenario: Parsed replies are the effective list in order

- **GIVEN** a settled oneshot whose parse produced three distinct reply strings
- **WHEN** the effective default-prompt list is built
- **THEN** the list is exactly those three strings in parse order

> test: code

### Scenario: Pre-oneshot list is the lifecycle heuristic when present

- **GIVEN** a session with no settled non-empty oneshot result
- **AND** a present lifecycle heuristic
- **WHEN** the effective default-prompt list is built
- **THEN** the list has exactly one entry
- **AND** that entry is the heuristic in empty-send form

> test: code

### Scenario: Failed or empty oneshot falls back to the heuristic

- **GIVEN** a settled oneshot that failed or produced no suggestions
- **AND** a present lifecycle heuristic
- **WHEN** the effective default-prompt list is built
- **THEN** the list has exactly one entry
- **AND** that entry is the heuristic in empty-send form

> test: code

### Scenario: No oneshot and no heuristic yields an empty list

- **GIVEN** a session with no settled non-empty oneshot result
- **AND** no lifecycle heuristic
- **WHEN** the effective default-prompt list is built
- **THEN** the list is empty

> test: code

## @ Requirement: Suggestion readiness

After a non-priming turn completes and a reply-suggestion oneshot is started for a
generation, suggestions SHALL be pending until that oneshot settles (success or failure)
for the same generation. While pending and the composer input is empty, the default-prompt
list SHALL NOT be presented and a loading indicator SHALL be shown instead; empty submit
SHALL NOT send a default prompt; Tab and Shift-Tab SHALL NOT cycle defaults. When the
oneshot settles for the matching generation, suggestions SHALL become ready: the effective
list is presented when non-empty and empty-input send and cycle are armed. Results for a
superseded generation SHALL NOT present or arm suggestions. When no oneshot is
outstanding, suggestions are ready (the effective list may be empty). While a main agent
turn is in progress (streaming), empty-input default prompts SHALL NOT be presented —
neither as a list nor as a loading indicator — regardless of oneshot readiness or whether
a non-empty effective list would otherwise be available.

### + Scenario: Main turn in progress hides default prompts

- **GIVEN** a main agent turn is in progress
- **AND** an empty composer input
- **AND** a non-empty effective default-prompt list would otherwise be available
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** the default-prompt list is not shown
- **AND** a defaults loading indicator is not shown

> test: code

## @ Requirement: Oneshot request framing

The reply-suggestion request SHALL carry the last assistant message, the preceding user
message when present, the project's discovered slash command names as priming hints, and
the lifecycle heuristic when the session has one. The heuristic SHALL be a soft hint only
— the oneshot MAY omit it, place it in any position, or invent other replies. When
embedding message bodies into the oneshot prompt, the assistant message SHALL be limited
to its last 40 lines and the user message SHALL be limited to its last 12 lines; when a
body is truncated, a truncation marker SHALL appear so earlier content is not implied to
be present. The instruction framing SHALL require 1–3 lines of the form `REPLY: <text>`
with this order when multiple lines are emitted: first line the most obvious continuation
of the flow; any middle lines alternatives; last line a negative or declining option when
a negative option is appropriate. An empty assistant message SHALL yield an empty
suggestion list without calling the model.

### + Scenario: Long assistant message is truncated to its last lines

- **GIVEN** a last assistant message longer than 40 lines
- **WHEN** the reply-suggestion request prompt body is built
- **THEN** the embedded assistant body keeps only the last 40 lines
- **AND** a truncation marker is present

> test: code

### + Scenario: Long user message is truncated to its last lines

- **GIVEN** a preceding user message longer than 12 lines
- **WHEN** the reply-suggestion request prompt body is built
- **THEN** the embedded user body keeps only the last 12 lines
- **AND** a truncation marker is present

> test: code
