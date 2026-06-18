# Live reconciliation and UI follow

Run `reconcile` from `reload_and_reconcile` whenever a change is archived during a
session, and re-point the selection and open editor for each reported move.

## Prerequisites

- [x] @step reconcile-returns-relocations

## Context

`refresh_after_move` is a free function over `area::ideas::State` and `tab_bar::TabState`,
both of which have `Default` impls. The follow scenarios are therefore tested directly:
construct a default ideas `State` and `TabState`, set `selected` (and a preview pinned tab
via `pinned_tab_id`) to an old path, call `refresh_after_move` with an `IdeaMove`'s paths
and title, and assert the selection and tab now reference the new path. Constructing the
full app `State` is impractical, so do not test the end-to-end `reload_and_reconcile`
wiring; the classification and reporting behind it are already covered by step 01.

## Tasks

- [x] 1. In `reload_and_reconcile`, after `state.project.reload()` and the
         externally-archived-change detection loop, call
         `idea_store::reconcile(&mut state.ideas.ideas, &state.project)`

- [x] 2. Loop the returned moves and call
         `area::ideas::refresh_after_move(&mut state.ideas, &mut state.tabs, ...)` for
         each; leave the function's `bool` return unchanged

- [x] 3. @spec ideas/reconcile Selection and editor follow relocations: The selected idea stays selected after relocation

- [x] 4. @spec ideas/reconcile Selection and editor follow relocations: The open idea editor tracks the idea after relocation
