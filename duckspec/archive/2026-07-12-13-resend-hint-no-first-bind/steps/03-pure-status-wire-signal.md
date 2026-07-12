# Pure status-wire signal

Close the review gap: the product distinction between “no stored id” and “stored but
unresumable” must be a pure, tested helper used by the status builder — not only an inline
boolean at the wire site.

## Prerequisites

- [x] @step pure-resend-hint-predicate
- [x] @step wire-statusinfo-and-meta-row

## Context

Review finding 1: `hint_hidden_when_stored_session_is_resumable` and
`hint_hidden_when_no_stored_agent_session_id` both call
`show_resend_history_hint(true, false)`, so a bad wire (`resumable_session_id().is_none()`
alone) still passes every unit test.

## Tasks

- [x] 1. Add a pure helper in `crates/duckboard/src/widget/agent_chat.rs`, e.g.
         `unresumable_stored_session(has_stored_agent_id: bool, will_resume: bool) -> bool`
         returning `has_stored_agent_id && !will_resume`; document that the status builder
         owns the mapping from session fields to these bools

- [x] 2. In `crates/duckboard/src/area/interaction.rs` status construction, set
         `unresumable_stored_session` via that helper
         (`has_stored_agent_id = agent_session_id.is_some()`,
         `will_resume = resumable_session_id().is_some()`)

- [x] 3. @spec chat/composer-footer Resend hint only for unresumable stored session: Hint hidden when stored session is resumable

- [x] 4. @spec chat/composer-footer Resend hint only for unresumable stored session: Hint hidden when no stored agent session id
