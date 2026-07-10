# Grok process-hot main and oneshot

Replace Grok cold adapters with long-lived ACP children on main and oneshot paths: process
reuse across calls, kill on cancel, N=1 oneshot session rotate.

## Prerequisites

- [x] @step worker-dual-path-and-handle-oneshots

## Context

Step 02 landed the dual main/oneshot worker loops. `spawn_worker` already calls
`ensure_hot` on first `RunTurn`, kicks oneshot warm-up, and invokes `rotate` after each
successful oneshot prompt. This step only needs to replace the Grok cold adapters in
`crates/duckchat/src/grok/runtime.rs` with process-hot `AcpTurn` holders — the worker
contract is already in place. Oneshot prompt text is fully assembled by the handle
(`build_title_prompt` / reply-suggest framing); runtimes return raw text only.

## Tasks

- [x] 1. Implement `GrokMainRuntime` holding an `Option<AcpTurn>`: `ensure_hot` spawns +
         `initialize` once; `run_turn` reuses the child for `open` + `prompt_events`; on
         cancel/kill clear the child so the next `ensure_hot` respawns; surface session
         ids as today.

- [x] 2. Implement `GrokOneshotRuntime` with a warm child, `prompt` collecting content
         deltas on the cheap model, and `rotate` opening a fresh ACP session (prefer
         `session/new` on the live child; fall back to respawn) after each successful
         prompt.

- [x] 3. Point `GrokProvider::open_main_runtime` / `open_oneshot_runtime` at the warm
         implementations; remove transitional cold adapters from step 01 if still present.

- [x] 4. Extend tests (scripted ACP peer and/or spawn counter) so process-reuse and
         oneshot isolation are observable without a live grok binary where possible.

- [x] 5. @spec harness/grok Session lifecycle and resume: A second turn on a hot path reuses the process

- [x] 6. @spec harness/grok Session lifecycle and resume: After cancel, the next turn can spawn and resume

- [x] 7. @spec harness/grok Warm oneshot path: A second oneshot call does not resume the prior oneshot session

- [x] 8. @spec harness/grok Warm oneshot path: An oneshot call on a hot path reuses the process
