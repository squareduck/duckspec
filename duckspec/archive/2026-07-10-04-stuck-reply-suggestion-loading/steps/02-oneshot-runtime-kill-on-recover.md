# Oneshot runtime kill-on-recover

Make Claude and Grok oneshot runtimes stop in-flight cheap-model work on drop/shutdown so
the worker’s err/timeout cold-reset path does not leave a wedged child blocking the next
Work.

## Prerequisites

- [x] @step worker-oneshot-budget-and-cold-reset

## Tasks

- [x] 1. Claude oneshot (`crates/duckchat/src/claude_code/runtime.rs`): hold a killable
         child (or equivalent); on timeout/drop/`shutdown`, kill the process instead of
         fire-and-forget thread + unkillable wait

- [x] 2. Grok oneshot (`crates/duckchat/src/grok/runtime.rs`): ensure abandoned/timed-out
         `prompt` and `shutdown` drop ACP child heat so the next `ensure_hot` can proceed

- [x] 3. Confirm worker cold-reset after `Err` still calls `shutdown` and that a
         subsequent oneshot on the same handle can complete against real runtime paths
         (unit/integration as available)

- [x] 4. Smoke-check that main-turn cancel remains independent of oneshot (cancel still
         does not require tearing down oneshot heat beyond existing warm-runtime rules)
