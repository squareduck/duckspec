# Classify unlinked scenarios in the change audit

Teach `duckpond::audit::audit_change` to split unlinked `test:code` change scenarios into
a pending bucket and an error bucket using step checkbox state.

## Tasks

- [x] 1. In `crates/duckpond/src/audit.rs`, add a `checked: bool` field to the internal
         `StepRef` struct and populate it in `collect_step_refs` from `task.checked` for
         task-level `SpecRef`s and `sub.checked` for subtask-level `SpecRef`s.

- [x] 2. Add a `pending_backlink_scenarios: Vec<ScenarioKey>` field to `AuditReport`,
         documented as informational (not counted by `total_errors()`); leave
         `missing_backlink_scenarios` and `total_errors()` unchanged.

- [x] 3. In `audit_change`, hoist `collect_step_refs` above the missing-backlink loop,
         build a "claimed" set from refs whose task is checked, and route each unlinked
         `test:code` change scenario to the error bucket when claimed or the pending
         bucket otherwise; sort the pending bucket by display key.
