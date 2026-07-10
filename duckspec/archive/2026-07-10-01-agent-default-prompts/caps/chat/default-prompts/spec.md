# Chat default prompts

Conversation-local empty-input defaults from a cheap-model oneshot: parse ordered `REPLY:`
suggestions (heuristic passed only as a soft hint), show and arm them only after the
oneshot settles, and drive empty Enter plus Tab cycling from that list alone.

## Requirement: Parsed suggestion list

Raw model output SHALL be reduced to at most three non-empty strings taken from lines that
start with the prefix `REPLY:`, in source order. Lines that do not match that prefix SHALL
be ignored. Text after the prefix is trimmed; empty results after trim are dropped.
Unknown slash forms (including command names not in any allow-list) SHALL be kept as
written.

> test: code

### Scenario: REPLY lines extracted in order and capped at three

- **GIVEN** model output with four lines each starting with `REPLY:` and non-empty text
- **WHEN** the suggestion list is parsed
- **THEN** the list has exactly three entries
- **AND** the entries are the first three reply texts in source order

### Scenario: No matching lines yields an empty list

- **GIVEN** model output with no line starting with `REPLY:`
- **WHEN** the suggestion list is parsed
- **THEN** the list is empty

### Scenario: Unknown slash text is preserved

- **GIVEN** model output with a `REPLY:` line whose text is an unknown slash form
- **WHEN** the suggestion list is parsed
- **THEN** the list contains that slash text unchanged

## Requirement: Oneshot request framing

The reply-suggestion request SHALL carry the last assistant message, the preceding user
message when present, the project's discovered slash command names as priming hints, and
the lifecycle heuristic when the session has one. The heuristic SHALL be a soft hint only
— the oneshot MAY omit it, place it in any position, or invent other replies. The
instruction framing SHALL require 1–3 lines of the form `REPLY: <text>` with this order
when multiple lines are emitted: first line the most obvious continuation of the flow; any
middle lines alternatives; last line a negative or declining option when a negative option
is appropriate. An empty assistant message SHALL yield an empty suggestion list without
calling the model.

> test: code

### Scenario: Heuristic is included in the request when present

- **GIVEN** a lifecycle heuristic for the session
- **WHEN** the reply-suggestion request is built
- **THEN** the request includes that heuristic as a soft hint

### Scenario: Ordering guidance is present in the instruction

- **GIVEN** the shared reply-suggestion instruction text

- **WHEN** the instruction is inspected

- **THEN** it requires first-line obvious continue, middle alternatives, and last-line
  negative when appropriate

### Scenario: Empty assistant yields empty list without a model call

- **GIVEN** an empty assistant message
- **WHEN** reply suggestions are requested
- **THEN** the suggestion list is empty
- **AND** no model call is made

## Requirement: Effective list is oneshot result only

The effective default-prompt list SHALL be exactly the parsed oneshot suggestion list
(order preserved, already capped). The lifecycle heuristic SHALL NOT be appended or merged
into the list after the oneshot. When the oneshot fails or returns no suggestions, the
effective list SHALL be empty.

> test: code

### Scenario: Parsed replies are the effective list in order

- **GIVEN** a settled oneshot whose parse produced three distinct reply strings
- **WHEN** the effective default-prompt list is built
- **THEN** the list is exactly those three strings in parse order

### Scenario: Failed or empty oneshot yields empty effective list

- **GIVEN** a settled oneshot that failed or produced no suggestions
- **AND** a present lifecycle heuristic
- **WHEN** the effective default-prompt list is built
- **THEN** the list is empty

## Requirement: Suggestion readiness

After a non-priming turn completes and a reply-suggestion oneshot is started for a
generation, suggestions SHALL be pending until that oneshot settles (success or failure)
for the same generation. While pending and the composer input is empty, the default-prompt
list SHALL NOT be presented and a loading indicator SHALL be shown instead; empty submit
SHALL NOT send a default prompt; Tab and Shift-Tab SHALL NOT cycle defaults. When the
oneshot settles for the matching generation, suggestions SHALL become ready: the effective
list is presented when non-empty and empty-input send and cycle are armed. Results for a
superseded generation SHALL NOT present or arm suggestions. When no oneshot is
outstanding, suggestions are ready (the effective list may be empty).

> test: code

### Scenario: Pending hides list and shows loading

- **GIVEN** a pending reply-suggestion oneshot
- **AND** an empty composer input
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** the default-prompt list is not shown
- **AND** a loading indicator is shown

### Scenario: Empty submit is a no-op while pending

- **GIVEN** a pending reply-suggestion oneshot
- **AND** an empty composer input
- **WHEN** the user submits
- **THEN** no message is sent

### Scenario: Ready after settle arms the effective list

- **GIVEN** a reply-suggestion oneshot that has settled for the current generation
- **AND** a non-empty effective default-prompt list
- **AND** an empty composer input
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** the effective list is shown
- **AND** empty submit sends the active entry

### Scenario: Superseded generation does not arm the list

- **GIVEN** a oneshot result whose generation no longer matches the session
- **WHEN** that result is applied
- **THEN** the session's ready default-prompt list is unchanged

## Requirement: Empty-input send and cycle

When the composer input is empty, suggestions are ready, and the effective default-prompt
list is non-empty, submit SHALL send the active entry of that list. When the list is empty
or suggestions are not ready, empty submit SHALL NOT send. Tab and Shift-Tab SHALL advance
and reverse the active index with wrap when the input is empty, suggestions are ready, the
list is non-empty, and slash-command completion is not consuming Tab. Cycling the active
entry SHALL NOT insert text into the composer input.

> test: code

### Scenario: Empty submit sends the active prompt

- **GIVEN** an empty composer input
- **AND** ready suggestions
- **AND** a non-empty effective default-prompt list
- **AND** an active index into that list
- **WHEN** the user submits
- **THEN** the sent text is the entry at the active index

### Scenario: Empty submit is a no-op when the list is empty

- **GIVEN** an empty composer input
- **AND** ready suggestions
- **AND** an empty effective default-prompt list
- **WHEN** the user submits
- **THEN** no message is sent

### Scenario: Tab cycles active index with wrap

- **GIVEN** an empty composer input
- **AND** ready suggestions
- **AND** an effective default-prompt list of at least two entries
- **AND** slash-command completion is not consuming Tab
- **WHEN** the user presses Tab at the last index
- **THEN** the active index wraps to the first entry
- **AND** the composer input remains empty
