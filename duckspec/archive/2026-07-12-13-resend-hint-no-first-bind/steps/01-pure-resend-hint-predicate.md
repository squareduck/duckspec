# Pure resend-hint predicate

Retarget `show_resend_history_hint` and its unit tests in `agent_chat.rs` so the footer
rule is non-empty transcript plus a stored but unresumable agent session.

## Tasks

- [x] 1. Change `show_resend_history_hint` in `crates/duckboard/src/widget/agent_chat.rs`
         to `show_resend_history_hint(has_messages, unresumable_stored_session) -> bool`
         returning `has_messages && unresumable_stored_session`; update the doc comment

- [x] 2. @spec chat/composer-footer Resend hint only for unresumable stored session: Hint shown when stored session is unresumable

- [x] 3. @spec chat/composer-footer Resend hint only for unresumable stored session: Hint hidden when stored session is resumable

- [x] 4. @spec chat/composer-footer Resend hint only for unresumable stored session: Hint hidden when transcript is empty

- [x] 5. @spec chat/composer-footer Resend hint only for unresumable stored session: Hint hidden when no stored agent session id
