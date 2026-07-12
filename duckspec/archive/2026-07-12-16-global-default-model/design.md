# Global default model - Design

Add a concrete global main-chat `ModelRef`, cascade pin → project override → global with
catalog presence as availability, block send when Missing, clear catalog slices on empty
rediscovery, and restructure Settings into global vs project.

## Approach

```
config.toml
  default_model: Option<ModelRef>     # global; seeded once catalog is ready
  model_defaults[hash]: ModelRef      # project override (unchanged map)

         pin?
          │yes
          ▼
     preferred ──project?── global ── (None if all unset)
          │
          ▼
   preferred ∈ process catalog?
      yes → Available(model)  → send OK, closed label = display name
      no  → Missing(preferred) → selector "Missing", send blocked
```

```
ModelCatalog.apply_harness
  non-empty → replace slice
  empty     → write empty slice   (no last-good keep)
```

```
Settings
├── Global
│   ├── fonts
│   ├── Default model          (concrete catalog choices only)
│   └── Chat (hints + oneshot rows for non-empty harnesses)
└── This project               (when root open)
    └── Default model override (“Use global default” sentinel)
```

Core type change: resolution stops inventing a model. Call sites that always had a
`ModelRef` take `EffectiveModel` (or `Option` + preferred) and branch on send / label /
meter.

## Config

`crates/duckboard/src/config.rs` — add global field beside `model_defaults`:

```rust
pub struct Config {
    // …
    /// Global main-chat default. `None` until seeded after catalog refresh
    /// (or user never had a choosable model).
    pub default_model: Option<ModelRef>,
    pub model_defaults: HashMap<String, ModelRef>,
    // …
}

impl Config {
    pub fn global_model_default(&self) -> Option<&ModelRef> { … }
    pub fn set_global_model_default(&mut self, model: Option<ModelRef>) { … }
    // project_model_default / set_project_model_default unchanged
}
```

Seed on first catalog-ready when `default_model` is `None`:

1. Prefer former built-in `("grok", "grok-4.5")` if present in catalog
2. Else first model from `available_models()` (catalog harness order)
3. Else leave `None` (everything Missing until a provider appears or user pins)

Persist after seed so Reset-to-defaults / next launch keep a concrete value when one was
choosable. `model_defaults` entries stay as project overrides with no rewrite.

## Cascade and effective model

`crates/duckboard/src/area/interaction.rs` — replace always-`ModelRef` cascade:

```rust
pub enum EffectiveModel {
    Available(ModelRef),
    /// Cascade produced a preferred choice that is not in the process catalog.
    Missing { preferred: ModelRef },
    /// No pin, no project override, no global (pre-seed or empty world).
    Unconfigured,
}

pub fn preferred_turn_model(
    pin: Option<&ModelRef>,
    project: Option<&ModelRef>,
    global: Option<&ModelRef>,
) -> Option<ModelRef> {
    pin.or(project).or(global).cloned()
}

pub fn resolve_effective_model(
    preferred: Option<&ModelRef>,
    in_catalog: impl Fn(&ModelRef) -> bool,
) -> EffectiveModel { /* … */ }
```

Remove `builtin_default_model` from the live cascade (keep the former pair only as seed
preference constants if useful).

Stamp both layers on sessions each tick (today only project):

```rust
// main.rs refresh_model_defaults
ax.project_model_default = config.project_model_default(root);
ax.global_model_default = config.default_model.clone();
```

Catalog membership: `available_models()` / `models_for_harness` contains matching
`(harness, id)`.

## Missing UX and send gate

Selector (`agent_chat::selected_model_choice` + status path in `interaction` view):

- `Available` → existing closed label (display name)

- `Missing { preferred }` → closed label **`Missing`** (synthetic choice; keep preferred
  in `id`/`harness` so equality stays stable); open list is still catalog models only —
  user must pick one to clear Missing

- `Unconfigured` → same **`Missing`** treatment (no preferred id)

Send paths (`SendPressed` → `send_prompt_text`, recovery re-dispatch, priming turn,
oneshot-hint chip send that starts a turn):

```rust
if !matches!(effective, EffectiveModel::Available(_)) {
    return; // blocked — no user message, no stream
}
```

Enter / send button do not dispatch. No auto-substitution. Usage meter: no fill when
Missing (`context_max = None`).

Agent subscription harness: use `preferred.harness` when preferred exists so a later
catalog fill / picker change still keys the worker; turns simply never leave the gate
while Missing.

## Model catalog (policy E)

`ModelCatalog::apply_harness` in `crates/duckboard/src/agent.rs`:

```rust
pub fn apply_harness(&self, harness: &str, discovered: Vec<ModelInfo>) {
    let mut map = self.by_harness.write().…;
    // Always write: non-empty replaces; empty clears prior slice.
    map.insert(harness.to_string(), discovered);
}
```

Update `harness/model-catalog` keep-last-good requirement → **clear on empty**. All
consumers (main pickers, oneshot, meter) share the new semantics. Single refresh-at-start
lifecycle unchanged.

## Settings layout

`crates/duckboard/src/area/settings.rs`:

```
| Section | Contents |
| --- | --- |
| **Global** | UI font, content font, **Default model** (concrete `model_entries()` only — no sentinel), Chat (hints + oneshots) |
| **This project** | Only when `project_root` is `Some`; **Default model** override with first choice `"Use global default"` (`id: None`) |
```

Messages: `GlobalModelSelected(ModelChoice)` always writes a concrete `ModelRef`;
`ModelDefaultSelected` stays project override (`None` clears map entry).

Oneshot rows: drop `ONESHOT_HARNESS_ORDER` as the iteration source. Iterate harnesses
present in the catalog with non-empty slices (stable order = `harness_rank` /
`ModelCatalog::all` order). Empty harness → no row (A). Oneshot **resolution** among a
harness’s models stays configured → string-match → first (unchanged).

## Picker helpers

`crates/duckboard/src/widget/agent_chat.rs`:

```rust
pub fn chat_model_choices() -> Vec<ModelChoice> { model_entries() }

/// Project override: sentinel first, then catalog.
pub fn project_override_model_choices(global: Option<&ModelRef>) -> Vec<ModelChoice> {
    // sentinel closed/open label: "Use global default"
    // optional: suffix resolved global display when Available
}

/// Global settings: catalog only, no sentinel.
pub fn global_model_choices() -> Vec<ModelChoice> { model_entries() }
```

Missing synthetic choice constructed at selection time, not as a permanent list entry.

## Impact

```
| Area | Change |
| --- | --- |
| `config.rs` | `default_model` field; get/set; serde default `None` |
| `interaction.rs` | `EffectiveModel`, cascade + catalog check; stamp global; block send |
| `main.rs` | seed on `ModelCatalogReady`; refresh stamps both defaults; subscription harness from preferred |
| `agent.rs` | catalog clear-on-empty; tests flip |
| `settings.rs` | Global vs project sections; global picker; oneshot iteration |
| `agent_chat.rs` | Missing closed label; project sentinel copy |
| Specs | `harness/selection` cascade; `harness/model-catalog` empty policy; `chat/oneshot-models` picker source; composer/selection Missing + blocked send as needed |
```

Migration: no rewrite of `model_defaults`; global seeds once; users with only project
defaults keep overrides and get a seeded global floor.

## Decisions

- **`EffectiveModel` tri-state** — Available / Missing / Unconfigured instead of always
  inventing `grok-4.5`. Alternative: always `Option<ModelRef>` and lose preferred identity
  for display (rejected: selector still needs preferred harness/id for synthetic Missing).

- **Seed on catalog-ready, not `Default` impl** — global stays `None` in pure defaults
  until models exist. Alternative: hardcode `grok-4.5` in `Config::default` (rejected:
  recreates the lie on machines without Grok).

- **Clear catalog on empty (E)** — one availability story for pickers and defaults.
  Alternative: keep last-good (rejected in explore).

- **Block at send path** — no-op return rather than disabling the text widget.
  Alternative: remove `on_submit` while Missing (equivalent UX; either fine if gate is
  centralized).

- **Oneshot still auto-resolves within a harness** — only main-chat refuses to decide.
  Oneshot rows simply absent when harness slice empty.

## Risks

- **Empty catalog after clear** → all chats Missing until a provider works; mitigated by
  seed when any model appears and by still allowing the user to open Settings once models
  return.

- **Subscription churn** — preferred harness with empty models still builds a worker;
  mitigated by existing warm-runtime laziness (no turn until send, which is blocked).

- **Spec churn on model-catalog** — keep-last-good scenarios invert; intentional product
  change, not silent test edits without delta.
