# Chat oversized messages

Oversized user chat text stays cheap to show: user cards render an unhighlighted source
prefix with a way to copy the stored full body, and the composer and queue skip highlight
without truncating.

## Requirement: Size gate

A body is oversized when it exceeds the oversized line cap or the oversized character cap,
counted on source lines and source characters.

> test: code

### Scenario: Over the line cap

- **GIVEN** a body whose line count exceeds the oversized line cap
- **AND** its source character count is within the oversized character cap
- **WHEN** the size gate is evaluated
- **THEN** the body is oversized

> test: code

### Scenario: Over the character cap on a single long line

- **GIVEN** a body of one line whose source character count exceeds the oversized
  character cap

- **WHEN** the size gate is evaluated

- **THEN** the body is oversized

> test: code

## Requirement: User display prefix

Oversized user transcript cards show a source prefix of the stored body, without syntax
highlight; the session keeps the full text.

> test: code

### Scenario: Oversized user card is an unhighlighted prefix

- **GIVEN** a session whose user message body is oversized
- **WHEN** the chat UI is materialized
- **THEN** the user card's display lines are a source prefix of that body
- **AND** the display lines are within the size gate
- **AND** the user editor has no syntax highlight
- **AND** the session still holds the full user message body

> test: code

### Scenario: Copy full writes the stored user body

- **GIVEN** a materialized oversized user card
- **WHEN** copy full is invoked for that card
- **THEN** the clipboard receives the full stored user message body

> test: code

## Requirement: Composer and queue highlight

Composer and queue keep their full buffers; highlight runs only when the buffer is not
oversized.

> test: code

### Scenario: Oversized composer and queue skip highlight, and a composer under the gate is highlighted

- **GIVEN** composer and queue buffers

- **WHEN** highlight is applied while each is oversized and again after the composer is
  within the size gate

- **THEN** the oversized composer and queue have no syntax highlight

- **AND** the under-gate composer has syntax highlight

> test: code

### Scenario: Theme rehighlight skips truncated user cards and oversized composer and queue

- **GIVEN** a truncated user card
- **AND** an oversized composer
- **AND** an oversized queue
- **WHEN** theme rehighlight runs
- **THEN** those editors have no syntax highlight

> test: code
