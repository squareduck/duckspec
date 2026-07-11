# Uniform ACP harness - Design

One harness-neutral ACP client in duckchat drives both Grok (native agent spawn) and
Claude (owned `duckchat-claude-acp` child that wraps the official `claude` CLI). The host
drops the Claude stream-json client path.

## Approach

```
duckboard  harness id → Provider
              │
              ▼
         duckchat worker
              │
              ▼
     AcpMainRuntime / AcpOneshotRuntime     ← single client stack
              │
              │  AgentLaunch { command argv }
     ┌────────┴────────┐
     ▼                 ▼
grok … stdio    duckchat-claude-acp
(native ACP)           │
                       ▼
                 claude -p duplex
                 (stream-json)
```

**Rule:** adapters only where the backend is not already ACP. Grok is spawned directly.
Claude gets a workspace agent binary. No Grok proxy.

**Dialect profile:** the client keeps the session/update shapes it already accepts from
Grok (`agent_message_chunk`, `agent_thought_chunk`, `tool_call` / `tool_call_update`,
`_meta.totalTokens`). The Claude agent **emits that profile**. One mapper in the client.

**Session ids:** Claude ACP `sessionId` = Claude Code’s native session id so persisted
duckboard resumes keep working across the cutover.

## Shared ACP client (`duckchat::acp`)

Lift and de-Grok from `crates/duckchat/src/grok/{acp,event,runtime}.rs`:

```rust
// crates/duckchat/src/acp/
//   turn.rs, event.rs, runtime.rs, launch.rs

pub struct AgentLaunch {
    /// Final argv already wrapped (login shell if needed). Client does not
    /// append harness-specific flags.
    pub build: Arc<dyn Fn() -> Command + Send + Sync>,
}

pub struct AcpTurn { /* child, reader, writer, next_id */ }

impl AcpTurn {
    pub async fn spawn_with(launch: AgentLaunch, cwd: &Path) -> Result<Self, Error>;
    pub async fn initialize(&mut self) -> Result<InitResult, Error>;
    pub async fn open(&mut self, session_id: Option<&str>, cwd: &Path)
        -> Result<String, Error>;  // new | load; SessionNotFound on dead load
    pub async fn prompt_events(/* … */) -> Result<PromptResult, Error>;
    pub async fn cancel(&mut self);
}

pub fn map_update(params: &Value, context_window: Option<usize>)
    -> Option<AgentEvent>;

pub struct AcpMainRuntime { /* ensure_hot, run_turn, shutdown */ }
pub struct AcpOneshotRuntime { /* ensure_hot, prompt, rotate N=1 */ }
```

Warm semantics stay as today for Grok: main process-hot across turns; cancel kills heat;
oneshot rotates logical session (N=1) while reusing process when possible. Both providers
open these runtimes with different `AgentLaunch` values.

Grok-only prompt knobs (`reasoningEffort`) remain optional parameters on `prompt`; Claude
launch simply never sets them.

## Providers (thin shells)

```rust
// ClaudeCodeProvider / GrokProvider
fn open_main_runtime(&self, wd: &Path) -> Box<dyn MainRuntime> {
    Box::new(AcpMainRuntime::new(self.launch(), wd))
}
fn open_oneshot_runtime(&self, wd: &Path) -> Box<dyn OneshotRuntime> {
    Box::new(AcpOneshotRuntime::new(self.launch(), wd))
}
```

```
| | Grok | Claude |
| --- | --- | --- |
| Launch | `grok --no-ask-user agent --always-approve stdio` (login-shell wrap) | `duckchat-claude-acp` (resolve binary; login-shell wrap if needed) |
| `list_models` | ACP initialize (cached) | static aliases for v1 (`opus`/`sonnet`/…); windows optional until agent advertises them |
| `list_commands` | shared `.claude` discover | same |
| `reasoning` | true | false |
| Title / reply | oneshot ACP + existing prompt helpers | same pattern; model pick stays harness-local |
```

Binary discovery for the Claude agent (in order): `DUCKCHAT_CLAUDE_ACP` env → sibling of
`current_exe()` → `PATH`. Missing binary → spawn error (same UX class as missing `grok`).

## Claude ACP agent (`duckchat-claude-acp`)

New workspace binary. Speaks ACP **server** on its stdio with duckchat; owns an inner
`claude` child.

```
parent (AcpTurn)  ←ACP JSON-RPC→  duckchat-claude-acp  ←stream-json→  claude
```

```rust
// crates/duckchat-claude-acp/
//   main.rs — stdio ACP agent loop
//   agent.rs — sessions, prompt, cancel
//   claude/{spawn,duplex,map}.rs

// Map Claude protocol lines → profile session/update values
fn claude_line_to_updates(msg: &ProtocolMsg) -> Vec<Value>;
```

```
| ACP | Agent behavior |
| --- | --- |
| `initialize` | protocol version; `loadSession: true`; curated models (match today’s aliases) |
| `session/new` | start duplex Claude; return Claude’s session id |
| `session/load` | resume that id; missing → error the client already maps to `SessionNotFound` |
| `session/prompt` | ACP content blocks → Claude user content; stream updates until stop |
| cancel / kill | tear down inner `claude`; parent may kill the agent |
```

Reuse host knowledge from `claude_code/run.rs`: stream-json in/out, disallowed tools,
`autoMemoryEnabled: false`, bypass permissions when policy says so, login-shell spawn.

**Claude heat:** production cutover uses process-hot duplex (`--input-format stream-json`
+ `--output-format stream-json`) so Claude main matches Grok warm behavior. Steps may
prototype the agent with cold `claude -p` + `--resume` for early ACP wiring tests; the
host cutover and deletion of the in-host stream-json client wait until duplex works.
**Emergency only:** if a duplex spike shows the CLI cannot hold a reliable multi-turn
stdio session, fall back to cold-inner inside the agent (client stays ACP; heat regresses
only on Claude) and record that in the change — do not plan cold-inner as the intended
ship.

Permissions: auto-approve / bypass in the agent; no duckboard permission UI (proposal
non-goal). Parent client keeps auto-answering agent→client JSON-RPC requests.

Attachments: client already builds ACP image/text blocks for Grok; agent translates those
into Claude content blocks (same idea as today’s attach encoding, moved server-side for
Claude).

## Host cleanup

After Claude provider uses ACP runtimes:

- Delete cold main/oneshot stream-json path used as the **client** (`claude_code/run.rs`
  turn driver in-host, cold `ClaudeMainRuntime` spawn-per-turn).

- Keep in duckchat only what providers still need: command discovery, title/reply prompt
  text helpers, harness id/`ModelInfo`.

- Protocol parsing / Claude spawn flags that only the agent needs move into
  `duckchat-claude-acp` (copy or shared small crate only if duplication hurts — prefer
  move once).

duckboard `agent.rs` harness match stays; no new harness ids. Subscription key still
includes harness.

## Impact

- New workspace member: `crates/duckchat-claude-acp` (+ binary packaging beside duckboard)

- `duckchat`: new `acp` module; `grok` / `claude_code` shrink to providers + launch

- Dev/run: build both binaries into the same `target/{debug,release}/` (workspace build or
  `cargo build -p duckchat-claude-acp -p duckboard`); discovery is env → sibling of
  `current_exe()` → `PATH` (no `CARGO_BIN_EXE` injection required)

- Persisted Claude sessions: keep working if agent uses native Claude session ids

- Specs: new Claude harness capability; warm-runtime / selection may only need light
  wording that both harnesses are ACP-hot

- No change to `ds init` harness list or slash-command install layout

## Decisions

- **No Grok ACP proxy** — direct spawn. Alternative: owned server per harness (rejected:
  double hop, zero semantic gain).

- **Own Claude agent binary, not npm/community agent** — control + no foreign runtime.
  Alternative: `claude-code-acp-rs` / npx adapters (rejected by proposal).

- **Adapter = CLI translator, not Messages-API agent** — remains Claude Code. Alternative:
  full tool loop in Rust (rejected: product rewrite).

- **In-tree `duckchat::acp` module first** — extract separate client crate only if a
  second consumer appears. Alternative: new `duckchat-acp` crate now (rejected: ceremony).

- **Client dialect = current Grok profile** — Claude agent conforms. Alternative: strict
  generic ACP + dual mappers (rejected: more host complexity).

- **Hand-rolled ACP on both sides for v1** — lift the existing client; implement the
  Claude agent server as matching hand-rolled JSON-RPC. Alternative: official
  `agent-client-protocol` crate on the server only (rejected for v1: version/dialect skew
  vs our client profile and extra dependency for little gain; revisit if the agent surface
  grows).

- **Claude models stay curated aliases in v1** — agent may advertise the same set on
  initialize. Alternative: require live model discovery (optional later).

- **Duplex at cutover, not cold-inner as the planned ship** — host flips to the Claude
  agent only once duplex main heat works. Cold-inner is an emergency fallback if the CLI
  cannot sustain duplex, not a deliberate intermediate release. Steps may still use
  cold-inner while scaffolding the agent. Alternative: ship ACP + cold-inner first and add
  duplex later (rejected: would meet architecture goals but miss warm parity that
  motivated the change).

- **Agent binary discovery: env → sibling of exe → PATH** — local dev builds
  `duckchat-claude-acp` into the same `target/` dir as duckboard so sibling resolution
  works; optional `DUCKCHAT_CLAUDE_ACP` override. Alternative: `CARGO_BIN_EXE` injection
  or a duckboard-only run that never builds the agent (rejected: fragile / silent miss).

## Risks

- **Duplex stream-json fragile or undocumented edge cases** → early spike; emergency
  cold-inner fallback inside the agent without abandoning ACP client; do not cut over the
  host until duplex is proven or fallback is an explicit recorded decision.

- **Session id / cwd key mismatch after cutover** → normalize cwd (existing
  `cwd::normalize_cwd`); use Claude’s native id as ACP sessionId; rely on existing
  SessionNotFound → clear + retry.

- **Agent binary not found in GUI / app bundle** → sibling-of-exe + env override; clear
  spawn error; ship/install both binaries together.

- **Dialect drift** (agent emits shapes client ignores) → shared golden fixtures: agent
  output must map through existing `map_update` tests.

- **Double failure mode** (agent up, claude missing) → map to explicit spawn/process
  errors; same operator mental model as missing grok.

- **Scope creep into reimplementing Claude** → hard boundary: no tool execution in our
  agent; only protocol translation.
