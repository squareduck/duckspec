# Raise oneshot call budget

Implement the oneshot call budget as thirty seconds of wall-clock time. Specs already name
the budget in scenarios; do not hard-code durations in GWT.

## Context

Delta: `caps/harness/warm-runtime` — requirement **Oneshot call budget and recovery**.
Production constant is `ONESHOT_CALL_BUDGET` in `crates/duckchat/src/worker.rs`.

## Tasks

- [x] 1. Set `ONESHOT_CALL_BUDGET` to `Duration::from_secs(30)` and update comments that
         state the production budget duration

- [x] 2. Leave injected short budgets in tests unchanged so CI does not wait the full
         production budget

- [x] 3. @spec harness/warm-runtime Oneshot call budget and recovery: Over-budget oneshot returns an error

- [x] 4. Run `cargo test -p duckchat` (oneshot / budget filters as needed)
