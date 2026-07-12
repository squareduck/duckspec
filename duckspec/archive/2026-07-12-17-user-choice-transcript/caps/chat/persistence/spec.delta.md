# @ Chat persistence

## + Requirement: User choice content

A chat session SHALL be able to store user-choice question and user-choice answer content
blocks alongside Text, Reasoning, and tool blocks. Persisting and reloading a session
SHALL preserve those blocks' bodies. A session file that contains only legacy content
kinds (no user-choice question or answer blocks) SHALL still load.

> test: code

### Scenario: User-choice question and answer blocks round-trip through persist and load

- **GIVEN** a session whose messages include a user-choice question content block and a
  user-choice answer content block

- **WHEN** the session is persisted and loaded again

- **THEN** the loaded session includes a user-choice question block with the same body

- **AND** the loaded session includes a user-choice answer block with the same body

> test: code

### Scenario: A legacy session without user-choice content still loads

- **GIVEN** a session file whose messages use only Text, Reasoning, ToolUse, and
  ToolResult content

- **WHEN** the session is loaded

- **THEN** the load succeeds

- **AND** the loaded messages match the file's content

> test: code
