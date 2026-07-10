# Warm agent runtimes — Design

Keep per-chat agent child processes warm so main turns, title summaries, and reply
suggestions skip spawn and handshake cost. Harness-agnostic runtimes live on the worker
behind `AgentHandle`; oneshots use a fresh logical session every call (N=1). Empty-input
defaults seed from (and fall back to) the lifecycle heuristic without a model call.

## Approach

```
  duckboard                              duckchat (per chat subscription)
  ─────────                              ────────────────────────────────
  Ready(handle)                          spawn_worker(provider, cwd, events)
       │                                      │
       │                                      ├─ MainRuntime     (cold until first send)
       │                                      └─ OneshotRuntime  (cold until first send)
       │
  first send ── handle.send_turn ──►     ensure_hot(main)
       │                                 (+ kick ensure_hot(oneshot) in background)
       │                                 run turn on main (stream events)
       │                                 process stays up
       │
  cancel ──── handle.cancel ────────►    kill main child; mark cold
                                         next send: ensure_hot again
       │
  after turn                             concurrent with idle main:
    handle.title_summary(req).await  ─►  oneshot queue (serialize)
    handle.reply_suggestions(req).await  ensure_hot → prompt → return
                                         then rotate session off hot path (N=1)
```

**Strategy:** Split “what models/commands exist” (`Provider`) from “a live process that
can run work” (`MainRuntime` / `OneshotRuntime`). The worker owns both runtimes for the
chat’s lifetime. Main and oneshot process command streams **concurrently** so a long main
turn does not block oneshot warm-up or (after the turn) title/reply. Title and reply share
one oneshot runtime and **serialize** with each other. Grok reuses real ACP children;
Claude implements the same traits as spawn-per-call no-ops.

Logical sessions stay separate from process heat:

```
| Path | Process | Logical session |
|------|---------|-----------------|
| Main | warm after first send | conversation id (resume across turns) |
| Oneshot | warm after first send | fresh every use (N=1); rotate after return |
```

## Runtime traits

New module `crates/duckchat/src/runtime.rs`. Harnesses implement these; the worker never
talks ACP or CLI flags directly.

```rust
// crates/duckchat/src/runtime.rs

/// Long-lived main-turn process (or cold equivalent). One per chat worker.
#[async_trait]
pub trait MainRuntime: Send {
    /// Spawn + handshake if cold. Idempotent when already hot.
    async fn ensure_hot(&mut self) -> Result<(), Error>;

    /// Run one turn. Caller supplies resume id via `req.session_id`.
    /// Streams into `events`. On cancel, kill the process and leave cold.
    async fn run_turn(
        &mut self,
        req: TurnRequest,
        events: mpsc::Sender<AgentEvent>,
        cancel: CancelToken,
    ) -> Result<TurnOutcome, Error>;

    /// Drop any held child. Safe if already cold.
    async fn shutdown(&mut self);
}

/// Long-lived cheap-model process for title + reply suggestions.
#[async_trait]
pub trait OneshotRuntime: Send {
    async fn ensure_hot(&mut self) -> Result<(), Error>;

    /// Single isolated prompt; returns raw assistant text (caller parses).
    /// Must not use tools. Does not resume a prior oneshot conversation.
    async fn prompt(&mut self, model_hint: OneshotKind, text: String) -> Result<String, Error>;

    /// Open a fresh logical session (or equivalent isolation) while keeping
    /// the process hot when possible. Called after each successful prompt (N=1).
    async fn rotate(&mut self) -> Result<(), Error>;

    async fn shutdown(&mut self);
}

/// Which cheap-model framing the oneshot is serving (model pick may differ later).
#[derive(Clone, Copy, Debug)]
pub enum OneshotKind {
    Title,
    ReplySuggest,
}

/// Factory: discovery-only provider opens stateful runtimes for a working dir.
pub trait Provider: Send + Sync {
    // existing: id, capabilities, list_models, list_commands …

    fn open_main_runtime(&self, working_dir: &Path) -> Box<dyn MainRuntime>;
    fn open_oneshot_runtime(&self, working_dir: &Path) -> Box<dyn OneshotRuntime>;
}
```

`Provider::run_turn` / `title_summary` / `reply_suggestions` as free-standing per-call
entry points go away (or become thin test helpers). Production path is always: factory →
runtimes → worker.

Shared framing stays in `reply_suggest` / title modules; the runtime only sees assembled
prompt text and returns raw model text. Parsing (`parse_replies`, `clean_title`) stays in
the worker (or thin handle helpers) so harnesses do not reimplement REPLY: rules.

```
  TitleRequest ──build_title_prompt──► oneshot.prompt(Title, text) ──clean_title──► String
  ReplySuggestionRequest
       ──build_reply_suggest_prompt + instruction──►
            oneshot.prompt(ReplySuggest, text) ──parse_replies──► Vec<String>
```

## Worker and AgentHandle

The worker still serializes **main** turns. It also owns a second command loop (or
`select!` branch) for oneshots so title/reply are not blocked by an in-flight turn, and so
first-send oneshot warm-up can run while the main turn streams.

```rust
// crates/duckchat/src/worker.rs

pub enum AgentCommand {
    RunTurn(TurnRequest),
    SetSessionId(String),
    ClearSessionId,
    Shutdown,
    // oneshot cmds are internal; public API is async methods on the handle
}

struct OneshotRequest {
    kind: OneshotKind,
    prompt: String,
    reply: tokio::sync::oneshot::Sender<Result<String, Error>>,
}

pub struct AgentHandle {
    cancel: CancelToken,
    tx: mpsc::UnboundedSender<AgentCommand>,
    oneshot_tx: mpsc::UnboundedSender<OneshotRequest>,
    working_dir: PathBuf,
}

impl AgentHandle {
    pub fn send_turn(&self, req: TurnRequest) { /* queue RunTurn */ }

    pub fn cancel(&self) { self.cancel.cancel(); }

    /// Awaitable oneshot: title summary via the shared oneshot runtime.
    pub async fn title_summary(&self, req: TitleRequest) -> Result<String, Error> {
        let text = build_title_prompt(&req);
        let raw = self.oneshot_call(OneshotKind::Title, text).await?;
        Ok(clean_title(&raw))
    }

    /// Awaitable oneshot: reply suggestions via the same runtime (serialized).
    pub async fn reply_suggestions(
        &self,
        req: ReplySuggestionRequest,
    ) -> Result<Vec<String>, Error> {
        if should_skip_model(&req) {
            return Ok(Vec::new());
        }
        let body = build_reply_suggest_prompt(&req);
        let text = format!("{REPLY_SUGGEST_INSTRUCTION}\n\n{body}");
        let raw = self.oneshot_call(OneshotKind::ReplySuggest, text).await?;
        Ok(parse_replies(&raw))
    }

    async fn oneshot_call(
        &self,
        kind: OneshotKind,
        prompt: String,
    ) -> Result<String, Error> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.oneshot_tx
            .send(OneshotRequest { kind, prompt, reply: tx })
            .map_err(|_| Error::Other("agent worker gone".into()))?;
        rx.await
            .map_err(|_| Error::Other("oneshot dropped".into()))?
    }
}

pub fn spawn_worker<P: Provider + 'static>(
    provider: P,
    working_dir: PathBuf,
    events: mpsc::Sender<AgentEvent>,
) -> AgentHandle {
    let mut main = provider.open_main_runtime(&working_dir);
    let mut oneshot = provider.open_oneshot_runtime(&working_dir);
    // main task: RunTurn / session id / cancel / shutdown
    // oneshot task: recv OneshotRequest → ensure_hot → prompt → send reply → rotate
    // on first RunTurn: also oneshot.ensure_hot() in background (best-effort)
    todo!()
}
```

**First-send warm:** On the first `RunTurn`, after (or as) `main.ensure_hot()`, the worker
fires a background `oneshot.ensure_hot()`. Title/reply after the turn then hit a hot
process. No duckboard-side pre-warm API.

**Cancel:** `cancel` only affects the main runtime. Oneshot work in flight fails or
finishes independently; cancel does not kill the oneshot child.

**Shutdown:** subscription end / `ProcessExited` path shuts down both runtimes.

```
  cmd_rx (main)                    oneshot_rx
       │                                │
       ▼                                ▼
  ensure_hot(main)                 ensure_hot(oneshot)
  run_turn ────────────────►       prompt ──► reply channel
  [kill if cancel]                 rotate()   // after return, N=1
```

## Grok warm runtimes

Today each call builds a fresh `AcpTurn` (`spawn_with` → `initialize` → `open` → `prompt`
→ `cancel`). Lifetime moves to the runtime:

```rust
// crates/duckchat/src/grok/runtime.rs (sketch)

struct GrokMainRuntime {
    spawn: Spawner,
    working_dir: PathBuf,
    turn: Option<AcpTurn>,
    /// Models from the last successful initialize (for context windows).
    models: Vec<AcpModel>,
}

impl MainRuntime for GrokMainRuntime {
    async fn ensure_hot(&mut self) -> Result<(), Error> {
        if self.turn.is_some() { return Ok(()); }
        let mut t = AcpTurn::spawn_with((self.spawn)(), &self.working_dir).await?;
        let init = t.initialize().await?;
        self.models = init.models;
        self.turn = Some(t);
        Ok(())
    }

    async fn run_turn(...) -> Result<TurnOutcome, Error> {
        self.ensure_hot().await?;
        // open(session_id) each turn: session/new or session/load
        // prompt_events on the held AcpTurn
        // on Cancelled / kill: self.turn = None
        todo!()
    }
}

struct GrokOneshotRuntime {
    spawn: Spawner,
    working_dir: PathBuf,
    turn: Option<AcpTurn>,
    session_id: Option<String>,
    models: Vec<AcpModel>,
}

impl OneshotRuntime for GrokOneshotRuntime {
    async fn ensure_hot(&mut self) -> Result<(), Error> {
        // spawn + initialize + session/new if cold or session missing
        todo!()
    }

    async fn prompt(&mut self, kind: OneshotKind, text: String) -> Result<String, Error> {
        self.ensure_hot().await?;
        // pick_title_model; session/prompt; collect ContentDelta text
        todo!()
    }

    async fn rotate(&mut self) -> Result<(), Error> {
        // Prefer: session/new on the live child, drop old session id.
        // Fallback: kill child + ensure_hot (still process-cold only once).
        todo!()
    }
}
```

**Main path change vs today:** same ACP sequence per turn (`open` + `prompt`), but
**without** re-spawn and **without** re-`initialize` when already hot. Cancel still
`start_kill`s the child (`AcpTurn::cancel`); runtime clears `turn` so the next
`ensure_hot` respawns.

**Oneshot N=1:** each `prompt` uses the current `session_id` (from the last `rotate` /
initial `session/new`). After the reply is sent to the handle caller, `rotate` opens a new
ACP session on the same child so the next title/reply does not see prior oneshot history.
Rotate failure leaves the runtime cold or marks it for re-`ensure_hot` on the next call —
never blocks the already-returned result.

**Model discovery:** `list_models` can keep using a short-lived handshake (or the memoized
`OnceLock` path in `GrokProvider`) — independent of chat runtimes.

## Claude no-op runtimes

Claude remains spawn-per-call behind the same traits. No efficiency claim in this change.

```rust
struct ClaudeMainRuntime { /* working_dir only */ }

impl MainRuntime for ClaudeMainRuntime {
    async fn ensure_hot(&mut self) -> Result<(), Error> { Ok(()) }

    async fn run_turn(...) -> Result<TurnOutcome, Error> {
        // existing claude_code::run::run_turn
        todo!()
    }

    async fn shutdown(&mut self) {}
}

struct ClaudeOneshotRuntime { /* working_dir */ }

impl OneshotRuntime for ClaudeOneshotRuntime {
    async fn ensure_hot(&mut self) -> Result<(), Error> { Ok(()) }

    async fn prompt(&mut self, kind: OneshotKind, text: String) -> Result<String, Error> {
        // spawn claude -p --model haiku with text; return stdout
        todo!()
    }

    async fn rotate(&mut self) -> Result<(), Error> { Ok(()) }

    async fn shutdown(&mut self) {}
}
```

## duckboard dispatch

Remove harness-matched cold oneshots in `main.rs`. After `TurnComplete`, use the session’s
`AgentHandle`:

```rust
// crates/duckboard/src/main.rs (sketch)

// title path
let handle = ax.agent_handle.clone().unwrap();
let work = async move {
    handle.title_summary(req).await.map_err(|e| e.to_string())
};
Task::perform(work, |result| Message::SessionTitleReady { key, result });

// reply path
let handle = ax.agent_handle.clone().unwrap();
let work = async move {
    handle.reply_suggestions(req).await.map_err(|e| e.to_string())
};
Task::perform(work, |result| Message::DefaultPromptsReady { key, prompts_gen, result });
```

No `GrokProvider::new()` / `ClaudeCodeProvider::new()` for oneshots. Harness choice
remains the subscription identity (`agent_subscription(..., harness)`); the handle’s
worker already runs the right provider’s runtimes.

## Pre-oneshot heuristic defaults

Empty-input defaults before a useful oneshot are **local to duckboard** — no
`OneshotRuntime`, no model call.

```
  obvious_command (lifecycle ladder: ds-explore | ds-spec | …)
        │
        ▼
  heuristic_prompt(cmd)  →  e.g. "/ds-explore"  (send form, leading slash)
        │
        ▼
  effective list when:
    • no oneshot has settled with non-empty replies for this session, or
    • oneshot fails / parses to empty
        │
        ▼
  ready immediately (not pending) when list non-empty

  oneshot settles with ≥1 REPLY:  →  effective list = parse only
                                     (heuristic not merged into that list)
```

`obvious_command` stays stored without a leading slash (today). The effective list entry
used for empty Enter is the slash-command form the composer already sends for skills
(`/{name}`). Exploration is not special-cased: whatever the ladder sets is the seed.

Implementation sketch in `crates/duckboard/src/default_prompts.rs`:

```rust
/// Build the send-form heuristic entry, or empty if none.
pub fn heuristic_as_prompts(obvious_command: Option<&str>) -> Vec<String> { todo!() }

/// Effective list: non-empty oneshot parse wins; otherwise heuristic fallback.
pub fn effective_prompts(
    oneshot_replies: &[String],
    obvious_command: Option<&str>,
) -> Vec<String> { todo!() }

/// On settle: Ok(non-empty) → that list; Ok(empty)|Err → heuristic fallback.
pub fn apply_oneshot_if_current(
    session_gen: u64,
    result_gen: u64,
    result: Result<Vec<String>, String>,
    obvious_command: Option<&str>,
) -> Option<Vec<String>> { todo!() }
```

Session creation / `refresh_obvious_command` should seed `agent_default_prompts` from the
heuristic so a brand-new chat is ready without waiting for a turn. While a oneshot is
pending, chrome stays loading (unchanged). Superseded generation still leaves the ready
list unchanged.

## Decisions

- **Two runtimes per chat (main + oneshot)** — isolation of model, cancel, and context.
  Alternatives: one shared process (rejected: cancel and model contention); global pool
  (rejected: proposal requires per-handle).

- **AgentHandle async methods for oneshots** — `title_summary` / `reply_suggestions` await
  results on the handle. Alternatives: only `AgentCommand` + raw oneshot channels at the
  call site (rejected: leaks worker plumbing into duckboard); stream results as
  `AgentEvent` variants (rejected: couples UI event loop to request/response pairing).

- **Concurrent main and oneshot queues** — oneshot can warm and run while main is idle or
  even during a turn’s wait for the next oneshot request after completion. Alternatives:
  single serial worker loop (rejected: first-turn oneshot warm blocked by long main turn).

- **Serialize title and reply on one oneshot runtime** — simpler N=1 rotate; title is
  once-per-chat. Alternatives: two specialized oneshot processes (rejected in proposal).

- **Lazy hot on first send** — `ensure_hot` on first `RunTurn`, plus background oneshot
  warm. Alternatives: hot on `Ready` (rejected: cost for unsent chats).

- **Kill on cancel** — main child dies; next send re-warms. Alternatives: soft protocol
  cancel keeping process (out of scope).

- **Claude as no-op hot** — same traits, spawn-per-call. Alternatives: invent long-lived
  Claude mode now (out of scope).

- **No idle teardown** — process lives until handle/subscription ends. Alternatives: idle
  timeout (deferred; not required for v1).

- **Pre-oneshot and failed-oneshot use heuristic as effective list** — best signal without
  a model; any ladder value, not explore-only. Alternatives: keep empty until oneshot
  (rejected: empty Enter useless on new chats); merge heuristic into non-empty oneshot
  results (rejected: oneshot remains sole list when it produces replies).

- **Failed/empty oneshot falls back to heuristic** — not empty list. Alternatives: empty
  on failure (rejected: hides the only remaining good default).

## Risks

- **Child dies between turns (OOM, crash)** → `ensure_hot` / `run_turn` treat I/O failure
  as cold and respawn once; surface error if respawn fails.

- **Rotate races the next oneshot** → if a second oneshot arrives before rotate finishes,
  either await rotate then prompt, or prompt on a freshly ensured session; never return
  prior oneshot history. Prefer await-in-flight rotate on the oneshot task (still off the
  UI hot path relative to main turns).

- **Many chats ⇒ many warm children** → bounded by open sessions’ subscriptions (already
  one worker per session). Accept for v1; idle eviction remains out of scope.

- **session/load after kill with stale id** → main still tracks conversation `session_id`
  in the worker; after kill+respawn, `open(Some(id))` resumes on a new child. If load
  fails (`SessionNotFound`), existing recovery path clears id and retries.

- **Trait-object runtimes vs monomorphized worker** → `Box<dyn MainRuntime>` keeps one
  worker implementation. Alternatives: generic worker over runtime types (more monomorphs,
  harder dual-queue). Prefer trait objects for the two runtimes; provider factory stays
  monomorphized at `spawn_worker` only if needed for construction.

## Open questions

- none (oneshot reply surface resolved: `AgentHandle` async methods)
