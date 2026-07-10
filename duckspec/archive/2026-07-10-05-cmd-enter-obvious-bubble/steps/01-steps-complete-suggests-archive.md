# Steps-complete suggests archive

When every step on a change is done, the lifecycle next command is `ds-archive` (not
`ds-review`), so orientation, soft oneshot hint, and the obvious bubble all point at
finalize.

## Tasks

- [x] 1. In `crates/duckboard/src/area/change.rs`, change `change_scope_facts` so
         `all_done` sets `next_command` to `ds-archive` instead of `ds-review`

- [x] 2. Update unit tests in `area/change.rs` that expected `ds-review` on all steps
         complete (obvious-command and orientation-related cases)

- [x] 3. @spec session/scope Lifecycle reflection: A change with all steps complete reports completion and the archive next-stage

- [x] 4. Confirm first-turn orientation text for an all-done change includes `/ds-archive`
         as the suggested next stage
