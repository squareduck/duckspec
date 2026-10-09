# Claude catalog and effort - Design

The Claude agent advertises the main rows of the Claude Code model catalog, including
per-model effort. Duckboard refreshes that slice at launch and at expiry, keeps the last
good snapshot when a fetch fails, and puts an effort pick beside the model on a Claude
chat. The next prompt restarts a hot `claude` whose model or effort no longer matches, and
resumes the same session.

## Catalog intake

`duckchat-claude-acp` fetches `https://downloads.claude.ai/model-catalog/v1/catalog.json`
with no auth during `initialize`. The host does not fetch it. Rejected: a duckboard-side
fetch, and a side file next to `initialize`.

The agent reads `surfaces.cc.model_selector_config[].models`. A row is advertised when
`section` is `main` and `min_claude_code_version` is absent or no newer than the `claude`
binary this agent spawns. Version is the leading `N.N.N` from that binary's `--version`,
compared component-wise. If the command fails or the triple cannot be parsed, rows that
require a minimum are dropped and rows without one stay.

The display name is the catalog `name`. The context window is `runtime.max_input_tokens`.
Effort is set only when `thinking.type` is `effort` and `runtime.effort_levels` is
non-empty: those ids in catalog order, plus `runtime.default_effort`. The recommended
badge and the effort description are not carried. Grok and Codex advertise no effort
field.

```rust
pub struct ModelEffort {
    pub default_level: String,
    pub levels: Vec<String>,
}

// added to ModelInfo and AcpModel
pub effort: Option<ModelEffort>,
```

```json
"_meta": {
  "totalContextTokens": 1000000,
  "effort": { "default": "medium", "levels": ["low", "medium", "high", "xhigh", "max"] }
}
```

The oneshot resolution ladder is unchanged. A main-row id such as `claude-haiku-5-5` still
matches the existing `haiku` needle. A saved bare `haiku` misses the exact match and falls
through that ladder.

## Refresh and last-good

Grok and Codex stay on one discovery per process. An empty rediscovery still clears that
harness slice. Claude does not.

`ClaudeCodeProvider` keeps a memo between refreshes and gains a force-refresh that runs a
new handshake. App start still refreshes every harness once. Later Claude refreshes are
Claude-only.

`initialize` always succeeds for a catalog failure. The shared ACP parser treats a missing
`catalog` object as `ok` with no expiry, so existing Grok and Codex peers stay normal
model lists. The Claude agent always sets the object.

```json
"_meta": { "modelState": { "catalog": { "status": "ok", "expiresAt": "2026-10-16T08:03:29Z" } } }
```

`status` is `ok` or `failed`. `expiresAt` is the document's RFC3339 `expires_at`, omitted
when absent. The curated alias table is not advertised.

```text
handshake error or status failed
        │
        ├── provider memo non-empty ──► keep memo
        ├── host slice already applied this process ──► keep slice
        ├── else snapshot file ──► install it, even past expiresAt
        └── else ──► Claude slice stays empty

status ok
        │
        ├── models non-empty ──► replace slice, replace memo, write snapshot
        └── models empty ──► replace slice and memo with empty; leave the file
```

A present map entry distinguishes "applied empty" from "never applied". `for_harness`
returns an empty vec for both, so the host uses map presence.

The snapshot is `~/.config/duckboard/claude-catalog.json`. It stores `expires_at` and the
advertised models (`id`, `display`, `context_window`, `effort`). The harness is
`claude-code` on load. A missing or unreadable file is no snapshot. Rows are installed as
stored. The next successful fetch filters again for the current binary.

A successful catalog with no `expiresAt` is valid for this process only. The next attempt
is the next launch.

While the app is open, one wake fires at `expiresAt` and fetches again. A separate
5-minute retry runs only while the Claude slice is empty or the held catalog is past
`expiresAt`. A fresh catalog does not poll. Both the wake and the retry call the
Claude-only refresh.

## Composer effort

The footer order is the model `pick_list`, then the effort `pick_list`, then the usage
readout. The effort control uses the same ghost style, text size, and padding as the model
control. It is shown only when the effective model is available and that row has effort.
Settings pickers are unchanged.

```rust
pub struct EffortPin {
    pub model: ModelRef,
    pub level: String,
}

// ChatSession, #[serde(default)] so older files load unpinned
pub effort_pin: Option<EffortPin>,
```

`None` means the chat has not chosen. The shown level is then `default_level`. Choosing
any offered level, including today's default, writes a pin for the preferred model. A
later catalog default change leaves a chat the user touched on the level they set.

Preferred model is the existing cascade: chat pin, project default, global default.
Reconcile the effort pin when session defaults are stamped, not in the view:

```text
preferred is None                         ──► keep pin
preferred identity == pin.model
    row available, level missing          ──► clear pin
    otherwise                             ──► keep pin
preferred identity != pin.model           ──► clear pin
```

A catalog blip that leaves the same preferred id in place does not clear the pin, and the
control hides while that model is Missing. A project or global default that changes what
an unpinned chat will run does clear it. The clear is persisted with the session.

Closed labels are `Low`, `Medium`, `High`, `Xhigh`, and `Max`. Any other id is shown
unchanged. The menu lists `levels` in catalog order and has no Default row.

## Hot apply

```rust
// TurnRequest
pub effort: Option<String>,
```

Every main-chat send sets `effort` to the level the composer resolved (pin, else catalog
default) when that row has effort, and leaves it `None` otherwise. The ACP client copies
it to `session/prompt` as `effort`. Rejected: overloading `ReasoningMode` /
`reasoningEffort`, which has no `xhigh` or `max` and is Grok's knob. Grok and Codex ignore
`effort`. Oneshot title and reply sends do not set it and do not share the main hot
process.

```rust
struct HotSpawn {
    duplex: ClaudeDuplex,
    model: Option<String>,
    effort: Option<String>,
}
```

The hot process records the model and effort it was spawned with. A prompt reuses it only
when the session id matches, the process is alive, and both strings match. Any other case
kills it and takes the existing cold path, which already `--resume`s a known native id.
`--effort` is passed when the prompt has a level and omitted when it does not. The ACP
session id the client holds does not change.

A picker change during an in-flight turn does not cancel that turn. The following prompt
applies the new flags.

```text
claude -p --resume <native id> --model claude-sonnet-5-5 --effort medium ...
```

## Compatibility

Older chats load with `effort_pin: None` and follow the catalog default. A chat pinned to
a model id that is no longer advertised shows the existing Missing closed label, and the
effort control hides until the preferred model changes or the id is advertised again.
Discovery failure no longer substitutes the alias table.
