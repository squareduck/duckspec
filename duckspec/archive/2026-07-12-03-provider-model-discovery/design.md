# Provider model discovery - Design

Unify model catalogs on provider discovery (ACP initialize for both harnesses), a
process-local cache refreshed at app start, provider-local display heuristics, and global
per-provider oneshot model settings wired into the worker oneshot path.

## Approach

```
app start
   │
   ▼
ModelCatalog.refresh()          duckboard process cache
   │  for each registered provider (shared instances)
   │     list_models() ──► discover (ACP initialize)
   │     map AcpModel → ModelInfo (display heuristic + window)
   │     on success: replace that harness’s slice
   │     on failure / empty: keep prior slice (or stay empty)
   ▼
pickers / settings / usage meter  ── read catalog only

chat worker spawn
   │
   ├─ main model: session pin / project default / built-in (unchanged)
   └─ oneshot preferred: config.chat.oneshot_models[harness]
                         else provider default match on catalog
                         → AcpOneshotRuntime.preferred_model
```

**Boundary:** duckchat providers own discovery + display mapping. duckboard owns the
process catalog, startup refresh, config, and settings UI. Claude’s **host** static list
goes away. The owned agent discovers the live Claude catalog (Anthropic Models API with
credentials available to the official `claude` install), advertises it on ACP
`initialize`, and falls back to curated aliases only when live discovery fails. Host never
calls Anthropic itself.

**Uniform surface:** `ModelInfo { harness, id, display, context_window }` — already the
picker/meter type; no duckboard string special-casing per harness.

## Process model catalog

Replace ad-hoc `ClaudeCodeProvider::new().list_models()` + Grok-only `OnceLock` in
`crates/duckboard/src/agent.rs` with shared provider instances and an explicit catalog:

```rust
// duckboard — sketch
pub struct ModelCatalog {
    // harness id → last good list (empty = none known yet / never succeeded)
    by_harness: RwLock<HashMap<String, Vec<ModelInfo>>>,
}

impl ModelCatalog {
    pub fn refresh(&self, providers: &[&dyn Provider]) { /* … */ }
    pub fn all(&self) -> Vec<ModelInfo> { /* flatten */ }
    pub fn for_harness(&self, harness: &str) -> Vec<ModelInfo> { /* … */ }
}

pub fn available_models() -> Vec<ModelInfo> { catalog().all() }
```

- **Refresh policy:** call `refresh` once at app start only (background / fire-and-forget
  OK; first paint may briefly see empty until ready). No re-refresh when Settings opens in
  this change.

- **Graceful fallback:** failed or empty discovery does **not** clear a previous good list
  for that harness; cold failure leaves that harness empty (same practical behavior as
  Grok today when binary missing).

- **Provider cache:** each provider keeps its own memo of the last successful discover
  (not one-shot `OnceLock` that can never update). `list_models` returns the memo;
  `refresh` forces rediscover.

## Provider discovery path

Both Claude and Grok use the same pattern Grok already has: spawn agent → `initialize` →
map models → cancel.

```rust
// Shared idea inside each provider (sketch)
fn discover_models(&self) -> Vec<ModelInfo> {
    // thread + current_thread runtime, never nest in caller's rt
    // AcpTurn::spawn_with → initialize → map → cancel
    // Err / empty → Vec::new() (caller decides whether to keep prior)
}

fn to_model_info(m: AcpModel) -> ModelInfo {
    ModelInfo {
        harness: HARNESS.into(),
        id: m.id.clone(),
        display: humanize_display(&m.id, &m.name), // provider-local
        context_window: m.context_window,
    }
}
```

```
| Provider | Discovery source | Catalog owner today → after |
| --- | --- | --- |
| Grok | `grok agent` ACP initialize | handshake (unchanged) + pass through windows/names |
| Claude | `duckchat-claude-acp` ACP initialize | **remove** static vec in `ClaudeCodeProvider::list_models`; host discovers via handshake only |
```

Claude agent (`crates/duckchat-claude-acp`):

1. On `initialize` (or a short-lived discover path used by initialize), call Anthropic
   `GET /v1/models` using credentials available to the official `claude` install (same
   auth surface Claude Code already uses — CLI has no `list-models` flag; the binary
   references this endpoint).

2. Map API models into ACP `availableModels` (`modelId`, human `name`,
   `totalContextTokens` in `_meta` when known).

3. On auth/network/parse failure, advertise a **curated alias fallback** (today’s
   fable/opus/sonnet/haiku set) so initialize still returns a usable list.

4. Host (`ClaudeCodeProvider`) only reads ACP initialize — never calls Anthropic and never
   owns a second static table.

`Provider::list_models` stays sync (existing trait); discovery still uses the
dedicated-thread handshake.

## Display name heuristics

Uniform field: always set `ModelInfo.display` before catalog insertion.

```rust
// Per provider — implementation detail, not a duckboard table
fn humanize_display(id: &str, advertised: &str) -> String {
    // If advertised is present and not equal to a raw/ugly id, prefer it.
    // Else transform id: strip vendor prefixes, title-case aliases, etc.
}
```

- **Claude:** prefer agent `name` when it looks human (“Opus 4.8”); else map known aliases
  (`haiku` → `Haiku`, `opus` → `Opus`, …) or light prettify of full ids.

- **Grok:** prefer advertised `name` (already good); light fallback for bare ids.

- UI (`agent_chat` closed label / menu) keeps using `display` only — no new harness
  branches.

## Oneshot model configuration

**Config** (`~/.config/duckboard/config.toml`, global):

```toml
[chat]
agent_input_hints = true

[chat.oneshot_models]
# harness id → model id from that harness’s catalog
"claude-code" = "haiku"
"grok" = "grok-composer-2.5-fast"
```

```rust
pub struct ChatConfig {
    pub agent_input_hints: bool,
    /// Global preferred oneshot model id per harness. Absent key → string-match default.
    pub oneshot_models: HashMap<String, String>,
}
```

**Resolution** (at worker spawn / oneshot open):

```rust
fn resolve_oneshot_model(
    harness: &str,
    configured: Option<&str>,
    catalog: &[ModelInfo], // that harness only
) -> Option<String> {
    // 1. configured id if still in catalog
    // 2. else provider default match (substring / ranked needles)
    // 3. else first catalog model
}
```

Default match needles (provider-owned constants or free functions):

```
| Harness | Prefer first match (case-insensitive) |
| --- | --- |
| `claude-code` | `haiku`, then first |
| `grok` | `composer`+`fast` / `fast`, then first |
```

**Wiring:** preferred model is no longer a private `TITLE_MODEL` constant baked only
inside `open_oneshot_runtime`. Pass it in when opening the oneshot runtime / spawning the
worker:

```rust
// Trait / spawn sketch
fn open_oneshot_runtime(
    &self,
    working_dir: &Path,
    preferred_model: Option<String>,
) -> Box<dyn OneshotRuntime>;

pub fn spawn_worker<P: Provider + 'static>(
    provider: P,
    working_dir: PathBuf,
    events: mpsc::Sender<AgentEvent>,
    oneshot_model: Option<String>, // new
) -> AgentHandle;
```

`drive_provider` / harness dispatch in `agent.rs` resolves `oneshot_model` from config +
catalog for the session harness before `spawn_worker`. Title + reply oneshots already
share `AcpOneshotRuntime` + `pick_oneshot_model` — keep that fallback (preferred if
advertised, else first).

Note: **title oneshots always run** even when `agent_input_hints` is off; they still use
the resolved oneshot model. The settings **pickers** appear only when oneshot affordances
are enabled (see below).

## Settings UI

In `crates/duckboard/src/area/settings.rs` Chat section:

- Keep **Agent input hints** toggler.

- When **on**, show one model `pick_list` **per harness that has catalog entries**, label
  by harness display (“Claude Code”, “Grok”), choices from `catalog.for_harness` (display
  names, store model id under `chat.oneshot_models[harness]`).

- When **off**, hide pickers; stored values and string-match defaults still apply to
  titles.

- Project **Default Model** section unchanged (still per-project main-chat default).

## Impact

- `duckchat`: Claude `list_models` → discover; provider model memo; `open_oneshot_runtime`
  / `spawn_worker` preferred-model parameter; remove hard `TITLE_MODEL` as sole source of
  truth (defaults remain as match needles).

- `duckchat-claude-acp`: live discover via Models API + Claude credentials; advertise on
  initialize with windows when known; curated alias fallback when live discover fails.

- `duckboard`: `ModelCatalog` + startup refresh; config `oneshot_models`; settings
  pickers; pass oneshot preference into worker spawn; `available_models` /
  `model_context_window` read catalog (Claude may gain windows).

- Caps likely touched later: `harness/claude` (oneshot preferred model / discovery),
  `harness/model-picker`, `harness/selection`, chat settings / default-prompts if oneshot
  gate behavior is specified there.

- Config migration: missing `oneshot_models` → defaults; no breakage of `model_defaults`
  or session pins.

## Decisions

- **Host discovers Claude via ACP initialize (mirror Grok)** — duckboard/duckchat host
  never calls Anthropic; existing client parser (`AcpModel` /
  `_meta.modelState.availableModels`) is the host boundary. Proposal non-goal “no public
  models API as primary catalog source” applies to the **host**, not the owned agent.

- **Claude live catalog inside the agent via Models API** — `duckchat-claude-acp` calls
  `GET /v1/models` with credentials available to the official `claude` install (CLI has no
  list-models flag; binary references this endpoint). Maps into ACP `availableModels` with
  names and windows when present. **Curated alias fallback** only when live discovery
  fails. Alternatives rejected: host calling Anthropic; curated-only for this change;
  non-existent CLI list flag.

- **Single Claude wire catalog in the owned agent; host only discovers** — eliminates dual
  hardcode in `claude_code.rs` + agent.

- **Process catalog with “keep last good on empty failure”** — better than pure `OnceLock`
  (no refresh, first empty stuck forever).

- **Catalog refresh at app start only** — single fire-and-forget refresh; no Settings
  re-refresh in this change.

- **Oneshot preference injected at worker spawn**, not read from disk inside duckchat —
  config stays in duckboard; duckchat stays harness-agnostic.

- **Settings pickers gated on agent input hints; resolution always applies** — pickers
  only when `agent_input_hints` is on; title oneshots still use the resolved oneshot model
  when chips are off.

- **Display heuristics stay provider-local** — uniform `ModelInfo.display` at the
  boundary.

## Risks

- **Startup discover spawns two agents (Claude ACP + grok)** → run refresh off the UI
  thread; timeout/fail soft; cache empty until success.

- **Configured oneshot model disappears from catalog after upgrade** → resolution falls
  back to default match then first; settings may show fallback selection.

- **Models API auth or network fails** → agent falls back to curated aliases so initialize
  still returns a list; host keeps last good cache when rediscover is empty.

- **Models API shape or alias mapping may not match Claude Code’s selectable aliases** →
  map full ids to aliases when possible; keep humanize_display for ugly ids; curated
  fallback for a known-good alias set.

- **Brief empty picker at first launch** → acceptable; optional: block only model menus
  until first refresh completes (not full app).
