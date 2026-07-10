# Runtime traits and provider factories

Add harness-agnostic `MainRuntime` / `OneshotRuntime` traits and wire both providers to
open them. Claude stays cold (spawn-per-call); Grok still cold-spawns in this step so the
API migrates without process-hot behavior yet.

## Tasks

- [x] 1. Add `crates/duckchat/src/runtime.rs` with `OneshotKind`, `MainRuntime`, and
         `OneshotRuntime` as in the design; export from `lib.rs`.

- [x] 2. Extend `Provider` with `open_main_runtime` and `open_oneshot_runtime`; remove or
         stop using free-standing `run_turn` / `title_summary` / `reply_suggestions` as
         the production path (thin wrappers OK only if needed for transitional tests).

- [x] 3. Implement Claude cold runtimes in `crates/duckchat/src/claude_code/` that wrap
         the existing `run::run_turn`, `title::title_summary`, and
         `reply_suggest::reply_suggestions` spawn paths; `ensure_hot` / `rotate` /
         `shutdown` are no-ops where appropriate.

- [x] 4. Implement Grok cold runtimes (or temporary adapters) that preserve today’s
         spawn-per-call behavior behind the new traits so `spawn_worker` can compile
         against both providers before step 03.

- [x] 5. `cargo test -p duckchat` (and fix any monomorphization / trait-object fallout
         from the Provider change).
