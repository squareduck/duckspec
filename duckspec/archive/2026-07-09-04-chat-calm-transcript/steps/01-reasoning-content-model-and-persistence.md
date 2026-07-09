# Reasoning content model and persistence

Add first-class Reasoning content blocks and a separate pending-reasoning buffer so
thoughts persist distinctly from answer text, including on eager mid-turn flushes.

## Tasks

- [x] 1. Add `ContentBlock::Reasoning(String)` in `crates/duckboard/src/chat_store.rs` and
         fix all exhaustive matches

- [x] 2. Add `ChatSession.pending_reasoning: String` (in-memory only; default empty on
         `new`)

- [x] 3. Fold non-empty `pending_reasoning` into a trailing `ContentBlock::Reasoning`
         message in `persist_session_snapshot` (and any other snapshot path that only
         folds `pending_text`)

- [x] 4. @spec chat/persistence Reasoning content: Reasoning content round-trips through persist and load

- [x] 5. @spec chat/persistence Reasoning content: A legacy session without Reasoning still loads

- [x] 6. @spec chat/persistence In-flight turn durability: Eager flush includes pending reasoning as Reasoning content
