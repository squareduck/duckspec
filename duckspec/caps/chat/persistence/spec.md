# Chat persistence

Chat sessions are written durably: each write is atomic, scope migration never overwrites
or drops a session, and an in-flight turn is persisted before any state mutation and
during the turn itself.

## Requirement: Atomic session writes

Persisting a session SHALL be atomic. An interrupted or failed write SHALL NOT replace the
previously-persisted file with truncated or partial content; the prior contents remain
readable until a write completes in full.

> test: code

### Scenario: A failed save leaves the prior contents intact

- **GIVEN** a session already persisted to disk
- **WHEN** a subsequent save of that session fails partway through writing
- **THEN** the session file on disk still parses as the previously-persisted session

> test: code
> - crates/duckboard/src/chat_store.rs:848

## Requirement: Non-destructive scope migration

Migrating a scope's sessions into another scope SHALL move every session and SHALL NOT
overwrite or discard one. When both scopes hold a session with the same id, the migration
SHALL keep the copy with more messages and SHALL preserve the other copy rather than
delete it.

> test: code

### Scenario: Migration into an occupied scope keeps both scopes' sessions

- **GIVEN** a source scope holding a session
- **AND** a target scope already holding a different session
- **WHEN** the source scope is migrated into the target scope
- **THEN** the target scope afterward holds both sessions

> test: code
> - crates/duckboard/src/chat_store.rs:880

### Scenario: Same-id collision keeps the fuller session and preserves the other

- **GIVEN** a source and target scope that each hold a session with the same id
- **AND** the source copy has more messages than the target copy
- **WHEN** the source scope is migrated into the target scope
- **THEN** the target's session for that id has the fuller set of messages
- **AND** the displaced copy is preserved rather than deleted

> test: code
> - crates/duckboard/src/chat_store.rs:911

## Requirement: In-flight turn durability

Messages streamed during a turn SHALL survive both a mutation of the scope's state and an
abrupt end of the turn. Before a scope's in-memory state is migrated, replaced, or
dropped, its sessions SHALL be persisted; and messages streamed within a turn SHALL be
persisted during the turn, not only when the turn completes.

> test: code

### Scenario: An in-flight turn survives a promotion

- **GIVEN** a session with messages streamed since its last persist
- **WHEN** the scope's in-memory state is migrated by a promotion
- **THEN** the persisted session includes those streamed messages

> test: code
> - crates/duckboard/src/chat_store.rs:945

### Scenario: Streamed messages are persisted before turn completion

- **GIVEN** a turn that has streamed messages and has not yet completed
- **WHEN** an eager flush occurs
- **THEN** the persisted session includes the messages streamed so far

> test: code
> - crates/duckboard/src/chat_store.rs:997

### Scenario: Eager flush includes pending reasoning as Reasoning content

- **GIVEN** a turn that has streamed reasoning into the pending reasoning buffer and has
  not yet completed

- **WHEN** an eager flush occurs

- **THEN** the persisted session includes that reasoning as Reasoning content

- **AND** that body is not stored as Text content

> test: code
> - crates/duckboard/src/chat_store.rs:1370

## Requirement: Reasoning content

A chat session SHALL be able to store Reasoning content blocks alongside Text and tool
blocks. Persisting and reloading a session SHALL preserve Reasoning bodies. A session file
that contains only legacy content kinds (no Reasoning) SHALL still load.

> test: code

### Scenario: Reasoning content round-trips through persist and load

- **GIVEN** a session whose messages include a Reasoning content block
- **WHEN** the session is persisted and loaded again
- **THEN** the loaded session includes a Reasoning block with the same body

> test: code
> - crates/duckboard/src/chat_store.rs:1140

### Scenario: A legacy session without Reasoning still loads

- **GIVEN** a session file whose messages use only Text, ToolUse, and ToolResult content
- **WHEN** the session is loaded
- **THEN** the load succeeds
- **AND** the loaded messages match the file's content

> test: code
> - crates/duckboard/src/chat_store.rs:1301

## Requirement: Last-known context usage

A chat session SHALL be able to store a last-known context usage total (tokens used for
the context meter). Persisting and reloading a session SHALL preserve that total. A
session file that omits context usage SHALL still load, with usage treated as zero.

> test: code

### Scenario: Context usage round-trips through persist and load

- **GIVEN** a session whose last-known context usage total is non-zero
- **WHEN** the session is persisted and loaded again
- **THEN** the loaded session has the same context usage total

> test: code
> - crates/duckboard/src/chat_store.rs:1174

### Scenario: A legacy session without context usage still loads

- **GIVEN** a session file that does not include a context usage field
- **WHEN** the session is loaded
- **THEN** the load succeeds
- **AND** the loaded session's context usage total is zero

> test: code
> - crates/duckboard/src/chat_store.rs:1197

## Requirement: Unsynced draft durability

A chat session SHALL be able to store an unsynced draft (the kept answer draft of a
cancelled turn). Persisting and reloading a session SHALL preserve it. A session file that
omits the field SHALL still load, with no unsynced draft.

> test: code

### Scenario: Unsynced draft round-trips through persist and load

- **GIVEN** a session holding an unsynced draft
- **WHEN** the session is persisted and loaded again
- **THEN** the loaded session holds the same unsynced draft

> test: code
> - crates/duckboard/src/chat_store.rs:1236

### Scenario: A legacy session without an unsynced draft still loads

- **GIVEN** a session file that does not include an unsynced draft field
- **WHEN** the session is loaded
- **THEN** the load succeeds
- **AND** the loaded session holds no unsynced draft

> test: code
> - crates/duckboard/src/chat_store.rs:1262

## Requirement: User choice content

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
> - crates/duckboard/src/chat_store.rs:1027

### Scenario: A legacy session without user-choice content still loads

- **GIVEN** a session file whose messages use only Text, Reasoning, ToolUse, and
  ToolResult content

- **WHEN** the session is loaded

- **THEN** the load succeeds

- **AND** the loaded messages match the file's content

> test: code
> - crates/duckboard/src/chat_store.rs:1079
