# CHANGE list rename and refresh UI

Wire rename and refresh affordances on exploration rows in the CHANGE list and connect
them to the helpers from step 01.

## Prerequisites

- [x] @step title-refresh-and-rename-helpers

## Tasks

- [x] 1. Add CHANGE-area messages and state for renaming an exploration (open inline edit,
         commit, cancel) and for requesting a title refresh on the selected/hovered
         exploration’s active session

- [x] 2. Surface rename and refresh controls on exploration rows in `view_list` (hover or
         selected affordances; disable or omit refresh when the session is streaming or
         has no summarizable content if that is already known cheaply)

- [x] 3. On rename commit, call the rename helper, `save_explorations`, and reconcile
         session display names for that scope

- [x] 4. On refresh, build the refresh title request from the active session, dispatch
         `title_summary` via the session’s `AgentHandle` oneshot path, and on
         `SessionTitleRefreshReady` apply with force-overwrite (ignore empty/error without
         clearing labels)

- [x] 5. Smoke-check in the running app or unit-level update tests that rename + refresh
         messages update exploration state as expected
