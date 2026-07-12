# Wire StatusInfo and meta row

Feed the pure predicate from a precise status flag and the meta-row view so first-bind no
longer flashes the resend hint.

## Prerequisites

- [x] @step pure-resend-hint-predicate

## Tasks

- [x] 1. On `StatusInfo` in `crates/duckboard/src/widget/agent_chat.rs`, replace
         `will_resume` with `unresumable_stored_session: bool` and update the field docs

- [x] 2. In `crates/duckboard/src/area/interaction.rs` status construction, set
         `unresumable_stored_session` to
         `ax.session.agent_session_id.is_some() && ax.resumable_session_id().is_none()`

- [x] 3. In the meta-row view, call
         `show_resend_history_hint(!session.messages.is_empty(), status.unresumable_stored_session)`;
         run `cargo test -p duckboard` and fix regressions
