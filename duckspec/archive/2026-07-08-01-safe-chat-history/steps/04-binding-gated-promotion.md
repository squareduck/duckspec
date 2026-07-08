# Binding-gated promotion

Gate `reload_and_reconcile` promotion strictly on the authoritative `pending_bindings`
entry, delete the focus-based fallback, and consume the binding so a reappearance cannot
re-promote.

## Prerequisites

- [ ] @step in-flight-turn-durability

## Tasks

- [x] 1. In `reload_and_reconcile`, promote a newly-detected change only when
         `pending_bindings.remove(&new_name)` yields an exploration id; drop the call to
         `fallback_exploration_id`.

- [x] 2. Delete `fallback_exploration_id` and any now-unused helpers it relied on.

- [x] 3. Confirm the binding is removed (consumed) by that promotion so a later detection
         of the same change directory finds no binding and does not promote again.

- [x] 4. @spec exploration/promotion Promotion requires an authoritative binding: Bound change adopts its originating exploration

- [x] 5. @spec exploration/promotion Promotion requires an authoritative binding: Unbound change adopts no exploration

- [x] 6. @spec exploration/promotion Bindings are single-use: A consumed binding does not re-promote on reappearance

## Outcomes

- The binding-gated decision was extracted from `reload_and_reconcile` into a
  `promote_bound_exploration(&mut State, new_name)` seam so the three scenarios can be
  tested against a real `State` (they set up an in-focus exploration and assert it is
  *not* adopted — the exact regression the focus-fallback removal fixes).
- Added a shared `crate::test_support` module (`FsTmp`, `with_home`, one `HOME_LOCK`) and
  moved `chat_store`'s test helpers onto it. `HOME` is process-global and
  `std::env::set_var` races concurrent readers; with the new promotion tests also
  mutating `HOME` from `main.rs`, a per-module lock would no longer be sufficient, so all
  HOME-touching tests now serialise through a single lock.
