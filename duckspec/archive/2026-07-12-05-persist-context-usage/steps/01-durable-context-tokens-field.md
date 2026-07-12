# Durable context tokens field

Add `context_tokens` on the durable chat session model so last-known meter usage survives
save and load, including legacy files without the field.

## Tasks

- [x] 1. Add `context_tokens: usize` to `ChatSession` and `PersistedSession` in
         `crates/duckboard/src/chat_store.rs` (`#[serde(default)]` on the persisted
         field), default `0` in `ChatSession::new`, and map it in `load_sessions_for` /
         `save_session`

- [x] 2. Fix any other construction sites that build `ChatSession` or `PersistedSession`
         without the new field

- [x] 3. @spec chat/persistence Last-known context usage: Context usage round-trips through persist and load

- [x] 4. @spec chat/persistence Last-known context usage: A legacy session without context usage still loads
