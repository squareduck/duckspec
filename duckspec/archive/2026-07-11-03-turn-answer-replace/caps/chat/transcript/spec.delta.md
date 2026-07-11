# @ Chat transcript

## @ Requirement: Segment construction

Contiguous same-kind assistant content SHALL coalesce into one Thinking, Activity, or
Answer segment; a kind switch SHALL open a new segment. While streaming, pending reasoning
and pending answer text SHALL appear on live Thinking / Answer segments rather than as
separate committed messages until flushed. When both pending reasoning and pending answer
text are open, the transcript SHALL present one live Thinking segment and one live Answer
segment (not multiple Answer segments for the same uncommitted draft).

> test: code

### + Scenario: Live reasoning with an open answer draft yields Thinking then one Answer

- **GIVEN** a streaming session with non-empty pending reasoning and non-empty pending
  answer text

- **WHEN** the transcript segments are built

- **THEN** the live segments include a Thinking segment then an Answer segment

- **AND** there is exactly one Answer segment for that open draft

> test: code

## + Requirement: Thinking body fade

Expanded Thinking body text SHALL use a text color that is more faded than Answer body
text in the same theme, while remaining legible. Thinking headers MAY use a more muted
color than the Thinking body.

### Scenario: Thinking body is more faded than Answer body

- **GIVEN** a transcript with an expanded Thinking segment and an Answer segment
- **WHEN** both bodies are presented in the chat UI
- **THEN** the Thinking body appears more faded than the Answer body
- **AND** the Thinking body remains legible

> manual: visual contrast in light and dark
