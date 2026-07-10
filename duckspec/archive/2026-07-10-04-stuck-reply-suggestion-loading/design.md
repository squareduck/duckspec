# Stuck reply-suggestion loading — Design

Bound every oneshot work item with a 10s wall-clock budget, cold-reset the oneshot runtime
on any error or timeout so the serial queue cannot wedge, and ensure duckboard always
leaves pending for ready so empty-input chrome cannot stick on `…`.

## Approach

```
  duckboard                         duckchat (per chat)
  ─────────                         ──────────────────
  TurnComplete
    begin_default_prompts_oneshot()   pending=true, gen++
    Task::perform ─────────────────►  handle.reply_suggestions(req)
         │                                   │
         │                            oneshot_call → queue Work
         │                                   │
         │                            oneshot loop (serial):
         │                              timeout(10s, ensure_hot + prompt)
         │                                 │              │
         │                              Ok(text)       Err(*)
         │                                 │              │
         │                              reply Ok       reply Err
         │                                 │              │
         │                              rotate (N=1)   shutdown → cold
         │                                                 (next Work: ensure_hot)
         │
  DefaultPromptsReady { gen, Ok|Err }
    apply_oneshot_if_current → list (or heuristic)
    pending = false

  ProcessExited / send clear
    pending = false (belt; gen bump on send)
```

**Strategy:** Fix the hang at the **worker oneshot boundary**. A UI-only watchdog would
clear chrome for one turn but leave title/reply Work stuck behind a hung `prompt()`, so
the next turn would hang again. A 10s per-call budget on each `Work` item returns an error
to the iced task; the loop then **always** cold-resets the oneshot runtime after any
failure (timeout or other `Err`) before accepting the next command. Duckboard already maps
oneshot `Err` to ready + heuristic via `apply_oneshot_if_current`; this change makes that
path reachable when the model would otherwise hang forever, and tightens lifecycle edges
that can leave `default_prompts_pending` true without a ready message.

Title and reply each get their **own** 10s budget (not a shared pool). They still
serialize; a slow title that times out still frees the path for reply.

## Oneshot work budget (worker)

`crates/duckchat/src/worker.rs` owns the serial oneshot loop. Wrap each `Work` body in
`tokio::time::timeout` with a single constant budget shared by title and reply-suggest.

```rust
// crates/duckchat/src/worker.rs

/// Wall-clock budget for one oneshot Work item (ensure_hot + prompt).
pub const ONESHOT_CALL_BUDGET: Duration = Duration::from_secs(10);

// inside oneshot loop, OneshotCommand::Work(req):
async {
    let result = match timeout(
        ONESHOT_CALL_BUDGET,
        async {
            oneshot.ensure_hot().await?;
            oneshot.prompt(req.kind, req.prompt).await
        },
    )
    .await
    {
        Ok(inner) => inner,
        Err(_elapsed) => Err(Error::Timeout("oneshot call exceeded budget".into())),
    };

    let ok = result.is_ok();
    let _ = req.reply.send(result);

    if ok {
        let _ = oneshot.rotate().await; // N=1, existing path
    } else {
        // Aggressive recover: any Err (including Timeout) drops heat.
        oneshot.shutdown().await;
        // next Work starts cold via ensure_hot
    }
}
```

`AgentHandle::oneshot_call` / `title_summary` / `reply_suggestions` signatures stay the
same; callers already surface `Result<_, Error>`. No separate cancel API is required for
this change: timeout is the hang bound; `shutdown` is the recover.

```
  Work N                Work N+1
  ──────                ────────
  ensure_hot
  prompt ── hang ──► timeout @ 10s
  reply Err(Timeout)
  shutdown → cold
                        ensure_hot (respawn)
                        prompt …
```

## Error surface

Add an explicit timeout variant so tests and logs distinguish hang recovery from spawn or
protocol failures.

```rust
// crates/duckchat/src/error.rs

pub enum Error {
    // existing: Spawn, Process, Protocol, SessionNotFound, Cancelled, Io, Other
    #[error("oneshot timed out: {0}")]
    Timeout(String),
}
```

Duckboard maps all `Err` arms the same way today (`apply_oneshot_if_current` → heuristic
ready). No special-case for `Timeout` in the UI is required; logging may mention timeout
explicitly.

## Runtime kill-on-recover

`OneshotRuntime::shutdown` must actually stop in-flight cheap-model work when the worker
calls it after timeout/err. Today Grok’s `drop_child` cancels the ACP turn; Claude’s
oneshot spawns a OS thread + child with no kill path if the async future is abandoned.

```rust
// crates/duckchat/src/runtime.rs — no trait change required if shutdown is sufficient

#[async_trait]
pub trait OneshotRuntime: Send {
    async fn ensure_hot(&mut self) -> Result<(), Error>;
    async fn prompt(&mut self, model_hint: OneshotKind, text: String) -> Result<String, Error>;
    async fn rotate(&mut self) -> Result<(), Error>;
    async fn shutdown(&mut self);
}
```

**Grok:** on worker `shutdown` after err/timeout, existing `GrokOneshotRuntime::shutdown`
→ `drop_child` is enough if `prompt` is cancelled/dropped when the timeout future drops.
Prefer driving `prompt` with a cancel token the worker can flip on timeout, or ensure
dropping the in-flight future kills the child before the next `ensure_hot`. Implementation
detail: make `prompt` abortable so the 10s timeout does not leave a zombie ACP session
consuming the process.

**Claude:** cold spawn-per-call. `prompt` must not leave an unkillable `claude` child
after timeout. Sketch: hold `Child` (or kill handle) on the runtime / join path;
`shutdown` and timeout drop kill the process; avoid fire-and-forget threads that outlive
the budget without a kill.

```rust
// claude oneshot — direction of travel
impl OneshotRuntime for ClaudeOneshotRuntime {
    async fn prompt(&mut self, _kind: OneshotKind, text: String) -> Result<String, Error> {
        // spawn child; race read-to-completion vs cancel/drop → kill child
        todo!()
    }

    async fn shutdown(&mut self) {
        // kill any held child; clear state
        todo!()
    }
}
```

Harness-specific kill quality is an implementation step under this design; the contract
is: after the worker’s err/timeout path returns, the next `Work` must be able to
`ensure_hot` and run without waiting on the previous call.

## Pending settle and lifecycle edges (duckboard)

Existing settle path stays the success model:

```rust
// crates/duckboard/src/main.rs — DefaultPromptsReady (unchanged shape)
let Some(list) = default_prompts::apply_oneshot_if_current(
    ax.default_prompts_gen,
    prompts_gen,
    result, // Ok or Err — both settle when gen matches
    ax.obvious_command.as_deref(),
) else {
    return Task::none(); // superseded: leave pending for the *current* gen
};
ax.agent_default_prompts = list;
ax.default_prompts_pending = false;
```

Gaps to close so pending cannot stick without a matching ready:

```
  Event                    pending effect
  ─────                    ──────────────
  begin oneshot            pending = true, gen++
  DefaultPromptsReady ok   pending = false (if gen match)
  DefaultPromptsReady err  pending = false (if gen match) — includes Timeout
  send_prompt_text         clear: gen++, pending = false
  ProcessExited            clear pending (and list) — NEW belt
  main AgentEvent::Error   do not require oneshot clear (oneshot independent)
```

```rust
// crates/duckboard/src/area/interaction.rs — AgentSession

impl AgentSession {
    pub fn clear_agent_default_prompts(&mut self) { /* gen++, clear list, pending=false */ }
    pub fn begin_default_prompts_oneshot(&mut self) { /* gen++, clear list, pending=true */ }
}

// ProcessExited handler (main.rs): also clear_agent_default_prompts() so chrome
// cannot show Loading after the worker is gone with no DefaultPromptsReady.
```

**Supersession:** if the user starts a new turn while a late oneshot is still finishing,
`clear_agent_default_prompts` bumps gen and sets `pending = false`; the late
`DefaultPromptsReady` is ignored. That is already correct. After the new turn completes,
`begin` sets pending again and a fresh oneshot Work is queued — and because the prior Work
either finished or was timed out and the runtime cold-reset, the queue can accept it.

**Pure helpers:** no change to `defaults_chrome` / `empty_submit_text` semantics. Timeout
is just another settle to ready. Optional unit test: “timeout-shaped Err settles like
other errors” can live next to existing `apply_oneshot_if_current` cases.

## Capability delta map

```
caps/
├── chat/default-prompts/     delta: pending ends on timeout/recovery as ready;
│                             empty chrome must not load forever after settle
└── harness/warm-runtime/     delta: each oneshot call bounded (10s);
                              any oneshot Err or timeout cold-resets oneshot heat;
                              later oneshot on same handle can run
```

## Decisions

- **Timeout in the worker, not UI-only** — UI timers clear chrome but leave the serial
  oneshot queue wedged. Alternatives: iced-side deadline only (rejected: next turn still
  hangs); unbounded wait (rejected: observed stuck `…`).

- **10s per Work item** — each title and each reply-suggest call gets a full 10s budget.
  Alternatives: 30–60s (rejected: too long with empty-input loading chrome); shared pool
  for title+reply (rejected: one slow title would starve reply unfairly without a clear
  win).

- **Aggressive cold-reset on any oneshot `Err` and on timeout** — after every failed Work,
  `shutdown` the oneshot runtime before the next command. Alternatives: reset only on
  timeout (rejected: protocol/spawn failures can leave a half-dead warm child that hangs
  the next call); keep heat on soft errors (rejected: harder recovery story).

- **Same settle path for timeout as other errors** — duckboard does not special-case
  `Timeout` for chrome; heuristic/empty ready applies. Alternatives: keep loading and
  retry (rejected: can thrash; out of scope for this fix).

## Risks

- **Healthy but slow oneshots hit 10s** → settle as failure + heuristic; user still gets a
  usable default (lifecycle command). Raise budget later if telemetry shows false
  timeouts.

- **Claude child not killed on timeout** → zombie process until exit; mitigate by holding
  a killable child in the oneshot runtime and killing on `shutdown`/drop.

- **Title times out first, reply still needs a slot** → acceptable: title failure is
  logged/ignored for chrome; reply Work runs next with its own 10s. Title remains optional
  for suggestions UX.

- **`shutdown` during rotate race** → only call rotate on Ok; on Err skip rotate and
  shutdown (design above).

## Open questions

None — budget is 10s per call; cold-reset is aggressive on any oneshot error and timeout.
