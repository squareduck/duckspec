# Post-implementation review: Rename changes in UI

Reviewed exploration list rename + title refresh end-to-end against proposal,
`exploration/list-labels`, steps, and duckboard code. Helpers and wiring are largely
right; cold-handle refresh and hollow integration tests block freeze.

## Scope

`proposal.md`, `caps/exploration/list-labels/{spec,doc}.md`, steps 01–02, and
`crates/duckboard/src/{chat_store,area/change,main}.rs`. No design.md.
Post-implementation; check/audit already green (6/6 linked).

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | fidelity | Refresh no-ops when agent handle is cold | /ds-step |
| 2 | major | quality | Refresh scenarios don’t hit the real apply path | /ds-step |
| 3 | minor | quality | Rename only via second-click; no list affordance | ignore |
```

## Findings

### 1. Refresh no-ops when agent handle is cold - fidelity/major

**Where:** `crates/duckboard/src/main.rs:5081-5083` (`start_exploration_title_refresh`)

**Why:** ↻ is shown for selected/hovered explorations even when `agent_handle` is `None`
(typical after reload with chat history). The click returns `Task::none()` with no
feedback — silent failure on the path retitle is meant to fix after a first (or stale)
title.

**Action:** Warm the oneshot path without a prior main turn (same capability title summary
already has when a handle exists), or gate/disable the control when refresh cannot run. Do
not leave a dead control.

### 2. Refresh scenarios don’t hit the real apply path - quality/major

**Where:** `crates/duckboard/src/chat_store.rs` refresh unit tests (~879–921) vs
production `apply_session_title_inner` / `SessionTitleRefreshReady` in `main.rs`

**Why:** Overwrite / empty / no-content tests compose pure helpers
(`apply_session_title_value` + `rename_exploration` / `title_refresh_target`) and never
call the integration that force-writes session title + exploration `display_name` +
persist. A regression that drops exploration rename on refresh still greens `@spec`
backlinks.

**Action:** Cover overwrite and no-op paths through the real apply/refresh entrypoints, or
extract one shared full-write helper used by both UI and tests.

### 3. Rename only via second-click; no list affordance - quality/minor

**Where:** `crates/duckboard/src/area/change.rs` `SelectChange` re-click (~508–521) and
exploration rows in `view_list`

**Why:** Refresh has a visible ↻; rename is discoverable only by re-clicking an already
selected exploration. Asymmetric and easy to miss; low compounding cost.

**Action:** Optional visible rename control (or document as intentional); otherwise leave
as-is.

## Verdict

Not ready to archive: fix cold-handle refresh and lock the refresh apply path in tests.
Second-click rename polish is optional.
