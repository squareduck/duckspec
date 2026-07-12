# Exploration soft-archive model

Add optional `archived_at` on duckboard explorations with load defaults and unit coverage
for soft-archive state.

## Tasks

- [x] 1. Extend `chat_store::Exploration` with optional `archived_at` (serde default /
         skip if none) and `is_archived()`

- [x] 2. Add a helper to stamp `archived_at` with local ISO-8601 time (same family as idea
         `created`) without deleting chat sessions

- [x] 3. @spec exploration/archive Soft archive state: Live exploration has no archive stamp

- [x] 4. @spec exploration/archive Soft archive state: Archiving stamps archive time and keeps chats

- [x] 5. @spec exploration/archive Soft archive state: Missing stamp loads as live
