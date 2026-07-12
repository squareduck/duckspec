# NewSession live-window insert

Close the review residual: NewSession should mint a missing interaction panel with
live-window equal half, not `or_default`.

## Prerequisites

- [x] @step wire-production-construction-and-force-show

## Context

Review finding: `crates/duckboard/src/area/change.rs` NewSession still uses
`interactions.entry(scope).or_default()`. SelectChange usually inserts first; this is
defense in depth for a bare insert.

## Tasks

- [x] 1. In `change::update` NewSession arm, replace `or_default()` with
         `or_insert_with(|| InteractionState::for_window(window_w))`

- [x] 2. Grep production panel inserts for remaining `InteractionState` + `or_default`;
         leave tests and door path alone
