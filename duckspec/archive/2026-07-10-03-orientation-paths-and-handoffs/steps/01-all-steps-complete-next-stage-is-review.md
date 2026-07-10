# All-steps-complete next stage is review

When every step on a change is done, `change_scope_facts` (and thus orientation and the
composer placeholder) suggest `/ds-review` instead of `/ds-archive`.

## Tasks

- [x] 1. In `crates/duckboard/src/area/change.rs`, set `next_command` to `"ds-review"`
         when all steps are complete (leave unfinished → `"ds-apply"`).

- [x] 2. Update unit tests and comments that still expect `ds-archive` for the
         all-steps-complete branch (including `facts_all_steps_complete_*` and
         `compute_obvious_command` / related assertions).

- [x] 3. Rename the test and `@spec` comment for the all-done lifecycle scenario to match
         the new scenario name (review, not archive).

- [x] 4. @spec session/scope Lifecycle reflection: A change with all steps complete reports completion and the review next-stage
