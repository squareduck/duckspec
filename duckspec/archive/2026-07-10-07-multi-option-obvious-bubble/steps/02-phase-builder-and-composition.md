# Phase builder and composition

Build phase-ranked lifecycle lists and gate/Commit rules into `ChangeScopeFacts` and pure
`build_obvious_chrome`, with unit tests for composition scenarios.

## Prerequisites

- [x] @step obviouschrome-pure-model

## Tasks

- [x] 1. Extend `ChangeScopeFacts` with ordered `lifecycle_commands` (bare names); set
         `next_command` to the first entry for orientation

- [x] 2. Implement phase ranks from the proposal (explore, propose, design/spec,
         caps→step/spec/archive, open steps→apply/review, all done→archive/review)

- [x] 3. Implement `build_obvious_chrome(scope, project, session_empty, vcs_dirty)`
         applying Confirm+Reject on nonempty active change sessions and Commit-only on
         archived + nonempty + dirty

- [x] 4. Add or update unit tests in `area/change.rs` (or colocated) for composition

- [x] 5. @spec chat/obvious-bubble Chrome composition: Caps without steps yield step then spec then archive

- [x] 6. @spec chat/obvious-bubble Chrome composition: All steps complete yield archive then review

- [x] 7. @spec chat/obvious-bubble Chrome composition: Nonempty change session includes Confirm and Reject

- [x] 8. @spec chat/obvious-bubble Chrome composition: Empty change session omits gate row

- [x] 9. @spec chat/obvious-bubble Chrome composition: Archived dirty nonempty session yields Commit only

- [x] 10. @spec chat/obvious-bubble Chrome composition: Empty exploration yields explore only
