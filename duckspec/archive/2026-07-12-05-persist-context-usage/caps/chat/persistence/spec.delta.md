# @ Chat persistence

## + Requirement: Last-known context usage

A chat session SHALL be able to store a last-known context usage total (tokens used for
the context meter). Persisting and reloading a session SHALL preserve that total. A
session file that omits context usage SHALL still load, with usage treated as zero.

> test: code

### Scenario: Context usage round-trips through persist and load

- **GIVEN** a session whose last-known context usage total is non-zero
- **WHEN** the session is persisted and loaded again
- **THEN** the loaded session has the same context usage total

### Scenario: A legacy session without context usage still loads

- **GIVEN** a session file that does not include a context usage field
- **WHEN** the session is loaded
- **THEN** the load succeeds
- **AND** the loaded session's context usage total is zero
