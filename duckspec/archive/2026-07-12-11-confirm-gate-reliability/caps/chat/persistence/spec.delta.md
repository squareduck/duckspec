# @ Chat persistence

## + Requirement: Unsynced draft durability

A chat session SHALL be able to store an unsynced draft (the kept answer draft of a
cancelled turn). Persisting and reloading a session SHALL preserve it. A session file that
omits the field SHALL still load, with no unsynced draft.

> test: code

### Scenario: Unsynced draft round-trips through persist and load

- **GIVEN** a session holding an unsynced draft
- **WHEN** the session is persisted and loaded again
- **THEN** the loaded session holds the same unsynced draft

> test: code

### Scenario: A legacy session without an unsynced draft still loads

- **GIVEN** a session file that does not include an unsynced draft field
- **WHEN** the session is loaded
- **THEN** the load succeeds
- **AND** the loaded session holds no unsynced draft

> test: code
