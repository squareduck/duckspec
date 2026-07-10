# Gate confirm when archive in lifecycle

Extend the Confirm/Reject gate so a non-empty active-change session shows the gate when
composed lifecycle includes `/ds-archive`, and cover the all-steps-complete composition
scenario.

## Context

No design for this change. Gate logic lives in `build_obvious_chrome` in
`crates/duckboard/src/area/change.rs` (active `Scope::Change` arm after lifecycle is
built). Today Confirm+Reject runs when `!session_empty && (has_review || !has_steps)`. Add
a third condition: any lifecycle option is `/ds-archive` (after
`format_lifecycle_command`). Empty sessions stay lifecycle-only. Existing composition
tests for open steps without review, empty session, and pre-step/post-review gates should
still pass; add a nonempty all-steps-complete case that asserts gate + archive/review
lifecycle.

## Tasks

- [x] 1. In `build_obvious_chrome`, after building `lifecycle`, set Confirm+Reject when
         the session is non-empty and any of: `has_review`, `!has_steps`, or lifecycle
         contains `/ds-archive`; update the nearby comment to match

- [x] 2. Add a unit test for all steps complete, no reviews, non-empty session: lifecycle
         `/ds-archive` then `/ds-review`, affirm Confirm, decline present

- [x] 3. Run the duckboard change-area / composition tests (or crate tests that cover
         `build_obvious_chrome`) and fix any regressions

- [x] 4. @spec chat/obvious-bubble Chrome composition: All steps complete nonempty session includes Confirm and Reject
