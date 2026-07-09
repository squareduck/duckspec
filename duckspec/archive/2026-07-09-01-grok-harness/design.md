# Add grok as a second agent harness — Design

A `GrokProvider` implementing duckchat's existing `Provider` trait over `grok agent stdio`
(ACP), a harness dimension threaded through model identity and persistence, and
harness-aware dispatch and picker in duckboard — with each chat session bound to a single
harness.

## Approach

The `Provider` trait, `AgentEvent` stream, and worker are already provider-neutral; the
only concrete impl is `ClaudeCodeProvider`. This change adds a second impl and opens the
construction seams that hardcode it.

```
   duckboard picker
   selects (harness, model)
          │
          ▼
   agent_subscription(key = chat_id + harness)      ← respawns when harness changes
          │
          ▼
   agent_stream: match harness
     ├── "claude-code" → spawn_worker(ClaudeCodeProvider) → claude -p        (per turn)
     └── "grok"        → spawn_worker(GrokProvider)       → grok agent stdio  (per turn)
                                   │
                    ACP: initialize → session/load|new → session/prompt
                                   │
                    session/update ─────────────▶ neutral AgentEvent
                    _meta.totalTokens + window ──▶ UsageUpdate
```

A chat session is bound to one harness because `session_id` and the title model are
harness-specific — a grok session id cannot `session/load` under Claude. The model may
vary within a harness; switching harness starts a fresh agent-side session (duckboard's
own transcript is preserved by `chat/persistence`).

The grok turn lifecycle mirrors Claude's per-turn `claude -p` + `--resume`: spawn
`grok agent stdio`, `initialize`, then `session/load(session_id)` when resuming or
`session/new` for a fresh session, then `session/prompt`. Cross-process resume via
`session/load` is confirmed working.

## Harness identity & model ref

`ModelInfo` gains the owning harness so aggregated model lists stay unambiguous. A
`ModelRef` pairs a harness id with a model id and becomes the persisted unit of model
choice. Legacy persisted values (a bare model-id string) deserialize as the `claude-code`
harness.

```rust
// crates/duckchat/src/provider.rs
pub struct ModelInfo {
    pub harness: String,          // NEW: "claude-code" | "grok"
    pub id: String,
    pub display: String,
    pub context_window: Option<usize>,   // NEW: drives the usage meter denominator
}

// crates/duckchat/src/request.rs (or a small shared module)
#[derive(Clone, Serialize, Deserialize)]
pub struct ModelRef {
    pub harness: String,
    pub model: String,
}

impl ModelRef {
    pub fn parse_legacy(raw: &str) -> Self { todo!() } // bare id → claude-code harness
}
```

`TurnRequest` does **not** gain a harness field: the worker already owns exactly one
provider, so the harness is implied by which worker runs the turn. Harness identity lives
on `ModelRef`, the session, and the subscription key.

## GrokProvider

Implements `Provider` over the grok CLI. Capabilities flip `reasoning: true` (grok exposes
reasoning effort). `list_models()` and each model's `context_window` come from the ACP
`initialize` / `session/load` response
(`modelState.availableModels[].totalContextTokens`), not a separate shell-out.
`list_commands()` reuses the existing `.claude` scan — grok loads the same skills.

```rust
// crates/duckchat/src/grok.rs  (+ crates/duckchat/src/grok/)
pub struct GrokProvider { bin: PathBuf, models: OnceCell<Vec<ModelInfo>> }

#[async_trait]
impl Provider for GrokProvider {
    fn id(&self) -> &str { "grok" }
    fn capabilities(&self) -> Capabilities {
        Capabilities { streaming: true, tool_use: true, resume: true,
                        reasoning: true, slash_commands: true }
    }
    fn list_models(&self) -> Vec<ModelInfo> { todo!() }   // from ACP handshake, cached
    fn list_commands(&self, project_root: &Path) -> Vec<SlashCommand> { todo!() } // reuse .claude scan
    async fn run_turn(&self, req: TurnRequest, events: mpsc::Sender<AgentEvent>,
                      cancel: CancelToken) -> Result<TurnOutcome, Error> { todo!() }
    async fn title_summary(&self, req: TitleRequest, working_dir: &Path)
        -> Result<String, Error> { todo!() }             // cheapest available model
}
```

## ACP client

A per-turn JSON-RPC 2.0 session over the child's stdio. It spawns
`grok agent --always-approve stdio`, runs the handshake, sends one `session/prompt`, and
pumps notifications until the prompt's response arrives. Responses are matched by `id`;
`session/update` notifications are the event stream; any agent→client request (e.g.
permission) is auto-answered so the turn never deadlocks.

```rust
// crates/duckchat/src/grok/acp.rs
struct AcpTurn { child: Child, next_id: u64, stdin: ChildStdin, lines: Lines<...> }

impl AcpTurn {
    async fn spawn(cwd: &Path, cancel: &CancelToken) -> Result<Self, Error> { todo!() }
    async fn initialize(&mut self) -> Result<InitResult, Error> { todo!() }        // → availableModels, loadSession
    async fn open(&mut self, session_id: Option<&str>, cwd: &Path)
        -> Result<String, Error> { todo!() }                                       // session/load | session/new
    async fn prompt(&mut self, session_id: &str, text: &str, model: &str,
                    reasoning: Option<ReasoningMode>,
                    events: &mpsc::Sender<AgentEvent>) -> Result<TurnOutcome, Error> { todo!() }
    fn cancel(&self) { /* session/cancel + kill child */ }
}
```

Resume mechanics are confirmed: `initialize` advertises
`agentCapabilities.loadSession: true`, and a fresh process that calls
`session/load { sessionId, cwd, mcpServers }` recovers the prior conversation.

## Event mapping

grok's ACP `session/update` variants map onto the existing neutral `AgentEvent` enum. This
finally exercises `ReasoningDelta`, which the Claude path never emits.

```
grok session/update            →  duckchat AgentEvent
──────────────────────────────    ─────────────────────────────────────────
agent_message_chunk            →  ContentDelta { text }
agent_thought_chunk            →  ReasoningDelta { text }
tool_call                      →  ToolUse   { id: toolCallId, name: title, input: rawInput }
tool_call_update (completed)   →  ToolResult{ id: toolCallId, name, output: content }
params._meta.totalTokens       →  UsageUpdate(Usage { input_tokens, output_tokens,
  + model.context_window                          context_window })
session/new|load → sessionId   →  SessionIdUpdated { session_id }
session/prompt result          →  TurnComplete
error / stopReason != end_turn →  Error(msg)
```

## Harness dispatch (duckboard)

Three construction sites currently name `ClaudeCodeProvider` directly. Each becomes a
match on the session's harness. `spawn_worker<P>` is monomorphized per arm, so no
trait-object change is required. `available_models()` aggregates across both providers.

```rust
// crates/duckboard/src/agent.rs
pub fn available_models() -> Vec<ModelInfo> {
    let mut m = ClaudeCodeProvider::new().list_models();
    m.extend(GrokProvider::new().list_models());
    m
}

fn agent_stream(project_root: PathBuf, harness: String) -> impl Stream<Item = AgentEvent> {
    // match harness { "grok" => spawn_worker(GrokProvider::new(), ..),
    //                 _      => spawn_worker(ClaudeCodeProvider::new(), ..) }
    // ReasoningDelta is now mapped instead of dropped.
}

pub fn agent_subscription(key: String, project_root: PathBuf, harness: String)
    -> Subscription<(String, AgentEvent)> { todo!() }  // key includes harness → respawn on switch
```

The title-summary site (`crates/duckboard/src/main.rs:1598`) dispatches on the same
harness.

## Harness-aware picker, meter & default (duckboard)

`ChatSession.selected_model` becomes `Option<ModelRef>`; the picker groups models by
harness and surfaces the active harness next to the observed model. The usage meter
denominator uses the selected model's `context_window`. The default resolution cascade
(`interaction.rs:1393` / `1277`, fallback `main.rs:400`) resolves to `grok` / `grok-4.5`
when nothing is pinned, replacing today's `None` ("CLI picks").

```rust
// crates/duckboard/src/chat_store.rs
pub struct ChatSession { /* ... */ pub selected_model: Option<ModelRef> }

// default when neither per-chat pin nor project default is set
fn resolve_default() -> ModelRef { ModelRef { harness: "grok".into(), model: "grok-4.5".into() } }
```

`config.toml` project defaults and `PersistedSession` store a `ModelRef`; a deserialize
shim maps legacy bare strings through `ModelRef::parse_legacy`.

## Decisions

- **Harness bound per chat session** — the worker owns one provider for its lifetime.
  Alternatives: a `MultiProvider` that dispatches each `run_turn` by a per-turn harness
  field (rejected: `session_id`/resume and the title model are provider-specific; mixing
  them in one worker breaks resume semantics).

- **Per-turn `grok agent stdio` + `session/load`** — mirrors Claude's per-turn `claude -p`
  + `--resume`; confirmed working across processes. Alternative: one long-lived ACP
  process per session (deferred as a latency optimization; adds process-lifecycle and
  cancellation complexity).

- **`list_models()` and context windows from the ACP handshake** — the `initialize` /
  `session/load` response carries `availableModels` with `totalContextTokens`.
  Alternative: shell out to `grok models` (rejected: a second source of truth for the same
  data).

- **Match-arm dispatch, not `Box<dyn Provider>`** — two known providers, monomorphized per
  arm. Alternative: trait objects (rejected: needless churn to `spawn_worker`'s signature
  for no gain today).

- **`ModelRef { harness, model }` + legacy shim** — carries harness through persistence;
  bare-string pins load as `claude-code`. Alternative: a parallel `harness` column on the
  session (rejected: two fields that must stay in sync).

- **Reuse the `.claude` command scan for grok** — grok discovers and executes the same
  skills (verified: all 12 `ds-*` show in `grok inspect`). Alternative: parse
  `grok inspect` / grok plugins (rejected: extra surface, same result).

## Risks

- **grok binary or auth absent on the machine** → `GrokProvider::new()` / `list_models()`
  degrade gracefully (empty list, typed error) and the picker omits the grok group; never
  panic.

- **`ToolPolicy::Interactive` unsupported on the grok path** → the initial grok provider
  supports only `BypassAll` (`--always-approve`); an interactive permission bridge over
  ACP `session/request_permission` is deferred and documented, not silently ignored.

- **Per-turn process startup latency** → acceptable for chat cadence; the
  persistent-process optimization remains available behind the same `Provider` API without
  touching callers.

- **Wrong usage denominator** → always take `context_window` from the selected model in
  the ACP handshake, never from an incidental stream value.

## Open questions

- None outstanding. Resume mechanics, model/context-window discovery, title-model
  fallback, and command reuse were resolved during design.
