# Reset next-action index on list refresh

When `next_actions` is rebuilt from a new trailing `next` (or bootstrap), set
`next_action_idx` to 0 so Tab from a prior card does not stick.

## Prerequisites

- [x] @step next-action-composer

## Context

From followup `reviews/04-followup-next-action-index-reset.md`:
`refresh_next_actions` only clamps the index, so a Tabbed index from an earlier
trailing `next` can select a secondary action on the next card (e.g. `reject`
instead of `confirm`).

## Tasks

- [x] 1. In `refresh_next_actions`, reset `next_action_idx` to 0 when the rebuilt
         list differs from the previous one (or always after rebuild if simpler
         and correct); keep clamp only if still needed for edge cases

- [x] 2. Add a unit test: active index mid-list on an old list, refresh to a new
         list → active index is the first entry
