# Chat meta cards

Duckboard-local recognition of chat `write` and `next` meta cards in assistant markdown:
which quote runs count as cards, their line ranges, and which send tokens a trailing
`next` card contributes.

## Requirement: Card recognition

A meta card SHALL be a maximal contiguous run of blockquote lines (a line whose leading
content is `>` with an optional following space) whose first non-empty content line, after
trim, is exactly `**write**` or `**next**`. The card's kind SHALL be write or next
accordingly. The card SHALL include every blockquote line from the start of that run
through the last blockquote line before a non-blockquote line ends the run. Each
recognized card SHALL report an inclusive 0-based line range covering every line of that
run. A blockquote run whose first non-empty content is not exactly `**write**` or
`**next**` SHALL NOT be a meta card. Lines inside an open fenced code block (between
matching fence openers and closers) SHALL NOT be treated as blockquote lines for meta-card
recognition, even when they begin with `>`.

> test: code

### Scenario: Known-kind quote run yields a card with inclusive line range

- **GIVEN** assistant markdown whose only blockquote run starts with `> **next**` and
  continues with two more blockquote body lines

- **WHEN** meta cards are parsed from that markdown

- **THEN** exactly one meta card is produced

- **AND** the card's kind is next

- **AND** the card's inclusive line range covers exactly those three blockquote lines

> test: code

### Scenario: Ordinary blockquote is not a meta card

- **GIVEN** assistant markdown that contains a blockquote run whose first non-empty
  content is ordinary prose (not `**write**` or `**next**`)

- **WHEN** meta cards are parsed from that markdown

- **THEN** no meta card is produced for that run

> test: code

### Scenario: Known-kind line inside a fenced code block is not a meta card

- **GIVEN** assistant markdown that places a line `> **next**` only inside a fenced code
  block

- **WHEN** meta cards are parsed from that markdown

- **THEN** no meta card is produced from that line

> test: code

## Requirement: Trailing next actions

Trailing next actions SHALL come only from a `next` meta card that is trailing: its
inclusive line range ends at the last non-empty line of the source (blank lines after the
card are allowed). When no such trailing `next` card exists, the trailing-action list
SHALL be empty — including when a `next` card appears earlier in the message or only a
`write` card is trailing. From a trailing `next` card, each body line after the kind line
SHALL contribute at most one action: the send text is the content of the first inline code
span on that line (text between the first pair of single backticks); if that line has no
such span, the line SHALL be skipped. Optional text after the first code span, after trim,
is reason only and SHALL NOT be part of the send text. Actions SHALL appear in source
order. At most three actions SHALL be produced; further token-bearing body lines SHALL be
ignored.

> test: code

### Scenario: Trailing next card yields ordered send tokens

- **GIVEN** assistant markdown that ends with a `next` meta card whose body has two lines
  each containing a first inline code span with distinct tokens

- **WHEN** trailing next actions are extracted

- **THEN** the action list has exactly those two send texts in source order

> test: code

### Scenario: Non-trailing next card yields no actions

- **GIVEN** assistant markdown that contains a `next` meta card followed by non-blank
  non-blockquote content after the card

- **WHEN** trailing next actions are extracted

- **THEN** the action list is empty

> test: code

### Scenario: Actions capped at three in source order

- **GIVEN** a trailing `next` meta card whose body has four lines each with a first inline
  code span

- **WHEN** trailing next actions are extracted

- **THEN** the action list has exactly three entries

- **AND** those entries are the first three send texts in source order

> test: code

### Scenario: Body line without a token is skipped

- **GIVEN** a trailing `next` meta card whose body has a line with no inline code span
  between two lines that each have a first inline code span

- **WHEN** trailing next actions are extracted

- **THEN** the action list has exactly two send texts

- **AND** they are the tokens from the two token-bearing lines in source order

> test: code

### Scenario: Reason after the token is not part of send text

- **GIVEN** a trailing `next` meta card body line whose first inline code span is
  `confirm` and whose remaining text after that span is a non-empty reason

- **WHEN** trailing next actions are extracted

- **THEN** the corresponding send text is exactly `confirm`

- **AND** the reason text is not included in the send text

> test: code
