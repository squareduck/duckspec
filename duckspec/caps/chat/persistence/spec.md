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
> - crates/duckboard/src/chat_store.rs:756

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
> - crates/duckboard/src/chat_store.rs:788

### Scenario: Same-id collision keeps the fuller session and preserves the other

- **GIVEN** a source and target scope that each hold a session with the same id
- **AND** the source copy has more messages than the target copy
- **WHEN** the source scope is migrated into the target scope
- **THEN** the target's session for that id has the fuller set of messages
- **AND** the displaced copy is preserved rather than deleted

> test: code
> - crates/duckboard/src/chat_store.rs:819

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
> - crates/duckboard/src/chat_store.rs:853

### Scenario: Streamed messages are persisted before turn completion

- **GIVEN** a turn that has streamed messages and has not yet completed
- **WHEN** an eager flush occurs
- **THEN** the persisted session includes the messages streamed so far

> test: code
> - crates/duckboard/src/chat_store.rs:904
