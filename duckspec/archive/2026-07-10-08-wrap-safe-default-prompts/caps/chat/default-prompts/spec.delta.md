# @ Chat default prompts

## @ Requirement: Parsed suggestion list

Raw model output SHALL be reduced to at most three non-empty strings taken from lines that
start with the prefix `REPLY:`, in source order. Lines that do not match that prefix SHALL
be ignored. Text after the prefix is trimmed; empty results after trim are dropped.
Unknown slash forms (including command names not in any allow-list) SHALL be kept as
written. A soft character budget in the oneshot instruction SHALL NOT cause the parser to
truncate reply text — over-budget strings SHALL be kept in full after trim.

> test: code

### + Scenario: Reply longer than 100 characters is preserved in full

- **GIVEN** model output with a `REPLY:` line whose text after trim is longer than 100
  characters

- **WHEN** the suggestion list is parsed

- **THEN** the list contains that full reply text unchanged (no character truncation)

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
a negative option is appropriate. The instruction framing SHALL soft-ask that each REPLY
text be at most 100 characters; that budget SHALL NOT be enforced by truncating parsed
results. An empty assistant message SHALL yield an empty suggestion list without calling
the model.

### + Scenario: Length guidance is present in the instruction

- **GIVEN** the shared reply-suggestion instruction text
- **WHEN** the instruction is inspected
- **THEN** it soft-asks that each REPLY text be at most 100 characters

> test: code

## + Requirement: Defaults list presentation

When the empty-input defaults chrome presents the ready effective list, each suggestion
SHALL soft-wrap within the composer width. Each list row's height SHALL follow its wrapped
content so consecutive suggestion rows do not overlap. The full suggestion text SHALL
remain visible — the chrome SHALL NOT hard-truncate or ellipsize the displayed value for
length.

### Scenario: Long suggestion soft-wraps without overlapping the next row

- **GIVEN** a ready non-empty effective default-prompt list
- **AND** an empty composer input
- **AND** at least one suggestion whose text is wider than the composer pane
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** that suggestion's text soft-wraps within the composer width
- **AND** the following suggestion row does not overlap the wrapped text

> manual: iced layout — confirm no paint-through between consecutive default rows

### Scenario: Full suggestion text is visible for a multi-line row

- **GIVEN** a ready non-empty effective default-prompt list
- **AND** an empty composer input
- **AND** a suggestion that wraps to more than one visual line
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** the entire suggestion text is visible without ellipsis or hard clip

> manual: visual check that multi-line default rows show full text
