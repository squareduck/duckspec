# Cold and unavailable refresh

Stop silent dead ↻ when chat content exists but the agent handle is cold; keep true
no-ops for streaming and non-summarizable chats.

## Prerequisites

- [ ] @step full-write-refresh-helper-and-tests

## Context

Review finding 1: `start_exploration_title_refresh` returns `Task::none()` when
`agent_handle` is `None` while the control still shows. Handle is also missing until
`AgentEvent::Ready` after subscription start / `ProcessExited`. Hover-only refresh can
miss an interaction if the exploration was never selected.

Prefer pending-until-Ready over inventing a second oneshot stack outside the agent
subscription. Finding 3 (rename affordance) is out of scope.

## Tasks

- [ ] 1. On `RefreshExplorationTitle`, ensure the exploration scope has an interaction
      and sessions (same as select) so a never-selected row can still refresh
- [ ] 2. When summarizable content exists but `agent_handle` is cold: record a pending
      title refresh for that session and run it on `AgentEvent::Ready` (or equivalent),
      instead of a silent no-op; clear pending on cancel/scope removal if needed
- [ ] 3. Keep no-op (or hide/disable ↻) when streaming or when `title_refresh_target` is
      `None`; do not wipe existing labels
- [ ] 4. Unit or update-level coverage: content present + cold handle eventually applies a
      force title via the step-03 full-write path once Ready supplies a handle
