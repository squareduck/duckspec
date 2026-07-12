# Unsynced draft capture and durability

Add the persisted `unsynced_draft` session field and capture the in-flight answer draft at
both cancellation sites.

## Tasks

- [x] 1. Add `unsynced_draft: Option<String>` to `ChatSession` and `PersistedSession`
         (serde default) in `crates/duckboard/src/chat_store.rs`; wire it through
         `save_session` and session load

- [x] 2. Add `capture_unsynced_draft` to `crates/duckboard/src/area/interaction.rs` (stash
         non-empty `pending_text`; no-op when empty) and call it in
         `on_answer_thrash_trip` before `flush_all_pending`

- [x] 3. Call `capture_unsynced_draft` in the `CancelPressed` arm before `handle.cancel()`

- [x] 4. @spec chat/cancel-resync Draft capture on cancellation: Thrash trip captures the kept draft

- [x] 5. @spec chat/cancel-resync Draft capture on cancellation: User cancel captures the in-flight draft

- [x] 6. @spec chat/cancel-resync Draft capture on cancellation: Cancellation with no in-flight draft records nothing

- [x] 7. @spec chat/persistence Unsynced draft durability: Unsynced draft round-trips through persist and load

- [x] 8. @spec chat/persistence Unsynced draft durability: A legacy session without an unsynced draft still loads
