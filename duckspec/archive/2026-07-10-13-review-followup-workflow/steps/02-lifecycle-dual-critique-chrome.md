# Lifecycle dual-critique chrome

Keep `/ds-review` and `/ds-followup` on lifecycle chrome for open steps and when there are
no open steps; stop dropping critique chips when a review file already exists.

## Prerequisites

- [x] @step kind-prefixed-critique-create

## Tasks

- [x] 1. Update `change_scope_facts` in `crates/duckboard/src/area/change.rs` to the
         design arms: open → `apply, review, followup`; all done no reviews →
         `archive, review, followup`; no open + has review →
         `step, spec, review, followup, archive`; leave pre-step arms unchanged

- [x] 2. Retarget unit tests in `change.rs` that lock chrome lists (open steps,
         open+review, all-done, rework path) to the new ordered vectors and gate
         expectations

- [x] 3. @spec chat/obvious-bubble Chrome composition: Open steps with review yield apply only with gate

- [x] 4. @spec chat/obvious-bubble Chrome composition: No open steps with review yield step then spec then archive with gate

- [x] 5. Update remaining chrome composition tests for same-name scenarios whose lists
         gained followup (`All steps complete yield archive then review`,
         `Open steps yield apply then review without gate`,
         `All steps complete nonempty session includes Confirm and Reject`)

- [x] 6. Confirm `session/scope` orientation tests still pass (next_command remains first
         of each arm: apply / archive / step); adjust phase label strings only if
         assertions pin them

- [x] 7. Run `cargo test -p duckboard` for change/scope/obvious-bubble coverage
