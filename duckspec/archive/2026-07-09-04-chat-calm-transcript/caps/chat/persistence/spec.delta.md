# @ Chat persistence

## @ Requirement: In-flight turn durability

### + Scenario: Eager flush includes pending reasoning as Reasoning content

- **GIVEN** a turn that has streamed reasoning into the pending reasoning buffer and has
  not yet completed

- **WHEN** an eager flush occurs

- **THEN** the persisted session includes that reasoning as Reasoning content

- **AND** that body is not stored as Text content

> test: code

## + Requirement: Reasoning content

A chat session SHALL be able to store Reasoning content blocks alongside Text and tool
blocks. Persisting and reloading a session SHALL preserve Reasoning bodies. A session file
that contains only legacy content kinds (no Reasoning) SHALL still load.

> test: code

### Scenario: Reasoning content round-trips through persist and load

- **GIVEN** a session whose messages include a Reasoning content block
- **WHEN** the session is persisted and loaded again
- **THEN** the loaded session includes a Reasoning block with the same body

### Scenario: A legacy session without Reasoning still loads

- **GIVEN** a session file whose messages use only Text, ToolUse, and ToolResult content
- **WHEN** the session is loaded
- **THEN** the load succeeds
- **AND** the loaded messages match the file's content
