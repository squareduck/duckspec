# Worker oneshot budget and cold-reset

Bound each oneshot Work item to a 10s wall-clock budget, return `Error::Timeout` on
overrun, cold-reset oneshot heat on any failure, and prove both warm-runtime recovery
scenarios with the fake worker.

## Tasks

- [x] 1. Add `Error::Timeout(String)` in `crates/duckchat/src/error.rs` and re-export via
         crate surface as needed

- [x] 2. In `crates/duckchat/src/worker.rs`, add `ONESHOT_CALL_BUDGET`
         (`Duration::from_secs(10)`) and wrap each `OneshotCommand::Work` body
         (`ensure_hot` + `prompt`) in `tokio::time::timeout`

- [x] 3. On timeout map to `Error::Timeout`; after any Work `Err` (including timeout) call
         `oneshot.shutdown().await` before the next command; keep `rotate` only on success
         (N=1)

- [x] 4. Extend worker fakes/tests so oneshot can hang past the budget and so a second
         oneshot can run after a forced failure (prefer injectable short budget in tests
         if hard-coding 10s would slow CI)

- [x] 5. @spec harness/warm-runtime Oneshot call budget and recovery: Over-budget oneshot returns an error

- [x] 6. @spec harness/warm-runtime Oneshot call budget and recovery: Later oneshot succeeds after prior oneshot failure
