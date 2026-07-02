# Consume the binding at reconcile

Attribute a newly-appeared change folder to its originating exploration via the recorded
binding, falling back to the active area. Implements the design's *`route_promotion`* and
*`reload_and_reconcile` rewrite* components.

## Prerequisites

- [ ] @step capture-the-originating-session

## Tasks

- [x] 1. Implement `route_promotion(state: &mut State, exp_id: &str, new_name: &str)` in
         `crates/duckboard/src/main.rs`: look up the exploration's `idea_path`; `None` →
         `area::change::promote_exploration(...)`; `Some(p)` →
         `promote_idea_exploration(state, Path::new(&p), new_name)` then
         `explorations.retain(|e| e.id != exp_id)` and
         `chat_store::save_explorations(...)`.

- [x] 2. Implement `fallback_exploration_id(state: &State) -> Option<String>`: for
         `Area::Change`, the `selected_change` when `is_exploration_selected()`; for
         `Area::Ideas`, the selected idea's `frontmatter.exploration` when its
         `frontmatter.change.is_none()`; otherwise `None`.

- [x] 3. Replace the promotion branch in `reload_and_reconcile` (`main.rs:2862-2927`):
         find the new change name, resolve `exp_id` from
         `pending_bindings.remove(&new_name).or_else(|| fallback_exploration_id(state))`,
         and call `route_promotion` when present.

- [x] 4. Confirm the removed branch-B retain/save logic is fully subsumed by
         `route_promotion` — no duplicated exploration removal or `save_explorations` call
         left behind in reconcile.

- [x] 5. `cargo build -p duckboard` and `cargo test -p duckboard`; smoke-check the
         two-session scenario (idea session creates a change while a change-area
         exploration is also selected → the idea gets its `change` link).

## Outcomes

- `cargo build -p duckboard` and the full `cargo test -p duckboard` suite (177 tests) pass
  with the reconcile rewrite in place.

- The live two-session GUI reproduction was **not** executed — it needs the running
  `duckboard` window driving two concurrent agent-backed sessions, which can't be
  reproduced headlessly. The routing is verified by construction and by step 01's parser
  unit tests; a manual confirmation in the app is still worth doing before relying on it.
