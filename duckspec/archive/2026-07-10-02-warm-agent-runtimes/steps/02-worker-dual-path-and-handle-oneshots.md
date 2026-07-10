# Worker dual path and handle oneshots

Own main + oneshot runtimes in the per-chat worker, expose async oneshot methods on
`AgentHandle`, and cover the warm-runtime contract with a fake cold-capable provider.

## Prerequisites

- [x] @step runtime-traits-and-provider-factories

## Tasks

- [x] 1. Rewrite `spawn_worker` to open main and oneshot runtimes from the provider, run a
         main command loop (`RunTurn` / session id / cancel / shutdown) and a concurrent
         oneshot loop; on first `RunTurn`, `ensure_hot` main and kick oneshot `ensure_hot`
         in the background.

- [x] 2. Add `AgentHandle::title_summary` and `AgentHandle::reply_suggestions` that
         assemble prompts via existing `reply_suggest` / title helpers, call the oneshot
         path, parse results, and return; empty assistant still short-circuits without a
         model call.

- [x] 3. On main cancel, kill/end main heat only; leave oneshot path intact. On shutdown,
         shut down both runtimes. After each successful oneshot prompt, call `rotate`
         (N=1) before the next oneshot.

- [x] 4. Add a test-only fake provider/runtimes that record spawn/session/resume and
         serialize oneshot prompts; drive the worker through the handle without real CLIs.

- [x] 5. @spec harness/warm-runtime Per-chat handle ownership: Title summary is requested through the chat handle

- [x] 6. @spec harness/warm-runtime Per-chat handle ownership: Reply suggestions are requested through the chat handle

- [x] 7. @spec harness/warm-runtime Lazy activation: First turn succeeds without a prior pre-warm call

- [x] 8. @spec harness/warm-runtime Lazy activation: Oneshot after first send needs no separate pre-warm API

- [x] 9. @spec harness/warm-runtime Oneshot serialization and isolation: Title and reply suggestions run one at a time on the oneshot path

- [x] 10. @spec harness/warm-runtime Oneshot serialization and isolation: A second oneshot call does not resume the prior oneshot session

- [x] 11. @spec harness/warm-runtime Cancel and re-warm: After cancel, a later turn on the same handle can complete

- [x] 12. @spec harness/warm-runtime Cold-capable harnesses: A cold-capable harness serves title summary through the handle
