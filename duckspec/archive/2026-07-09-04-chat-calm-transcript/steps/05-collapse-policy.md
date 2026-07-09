# Collapse policy

Default and auto-collapse Thinking and Activity segments for live vs settled turns,
respecting a per-segment user toggle override.

## Prerequisites

- [ ] @step segment-builder-construction-and-pairing

## Tasks

- [x] 1. Track collapse state with a user-override bit (e.g.
         `CollapseState { collapsed, user_set }`) aligned to segment indices on
         `AgentSession`

- [x] 2. Apply first-sight defaults: live Thinking/Activity expanded; settled/reload
         Thinking and Activity collapsed; Answer not collapsible

- [x] 3. Auto-collapse untoggled Thinking when a following Answer appears or the turn
         completes; auto-collapse untoggled Activity when the turn settles

- [x] 4. @spec chat/transcript Collapse defaults: Thinking collapses when answer follows

- [x] 5. @spec chat/transcript Collapse defaults: User-expanded Thinking is not auto-collapsed

- [x] 6. @spec chat/transcript Collapse defaults: Settled Activity starts collapsed
