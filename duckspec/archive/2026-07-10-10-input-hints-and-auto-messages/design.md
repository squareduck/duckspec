# Input hints and auto messages — Design

Two global config flags gate agent under-input suggestions and obvious chip chrome;
empty-session under-input list is a pure recompute from `lifecycle[0]`, reusing the
existing defaults list chrome and Enter path.

## Approach

```
~/.config/duckboard/config.toml
  [chat]
  agent_input_hints = false   # default
  auto_messages     = true    # default
           │
           ├────────────────────────────┐
           ▼                            ▼
  TurnComplete oneshot gate      chrome_visible / ⌘ resolvers
  (agent_input_hints)            (auto_messages)

session.messages.is_empty()?
        │
   yes ─┴─▶ effective list = [lifecycle[0]]   ready, never pending
   no  ──▶ agent_input_hints?
              yes → settled oneshot parse only
              no  → []
        │
        ▼
  agent_chat defaults chrome (under input)
  empty Enter / Tab cycle  (unchanged UI)
```

Boundaries:

- **In:** `config` / Settings, pure `default_prompts` effective-list rules, oneshot launch
  in `main`, `chrome_visible` and key resolvers, wire-up in `interaction` view/send.

- **Out:** lifecycle ladder composition, oneshot request framing, ghost-in-field UI,
  renames of capability paths or `agent_default_prompts` field names.

- **Product names:** Settings labels and cap docs say **input hints** / **auto messages**;
  on-disk keys stay short and stable under `[chat]`.

## Config and Settings

Add a nested `chat` table on the existing global config (same file already holds fonts and
project model defaults). Defaults match the proposal: agent off, auto messages on.

```rust
// crates/duckboard/src/config.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: FontConfig,
    pub content: FontConfig,
    pub projects: ProjectsConfig,
    pub model_defaults: HashMap<String, ModelRef>,
    pub chat: ChatConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatConfig {
    /// Under-input agent (oneshot) suggestions after a turn.
    pub agent_input_hints: bool,
    /// Obvious lifecycle / affirm / decline chip chrome + ⌘ bindings.
    pub auto_messages: bool,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            agent_input_hints: false,
            auto_messages: true,
        }
    }
}
```

Serde `#[serde(default)]` on `Config` and `ChatConfig` keeps missing keys (existing
configs without `[chat]`) on the product defaults — no migration file.

Settings (`crates/duckboard/src/area/settings.rs`): a **Chat** section with two boolean
controls (iced `toggler` or equivalent button pair — first bool pattern in Settings).
Messages flip the flag, `config::save`, same pattern as font changes. Labels:

```
| Control | Default | Copy (short) |
|---------|---------|--------------|
| Agent input hints | off | Suggest replies under the empty composer after a turn |
| Auto messages | on | Show lifecycle action chips when the composer is empty |
```

Toggles apply immediately on next view/update — no restart. Disabling agent mid-session
clears armed oneshot state on the active path (see wire-up).

## Effective input hints

`crates/duckboard/src/default_prompts.rs` owns the pure list rule. Expand
`effective_prompts` so call sites stop treating oneshot storage as the full story.

```rust
// crates/duckboard/src/default_prompts.rs

/// Build the under-input list for empty composer.
///
/// - Empty session: single entry from first lifecycle option (formatted), or empty.
/// - Non-empty session + agent hints on: settled oneshot parse only.
/// - Non-empty session + agent hints off: empty.
/// Never merges disk seed with oneshot results.
pub fn effective_prompts(
    session_empty: bool,
    first_lifecycle: Option<&str>,
    oneshot_replies: &[String],
    agent_input_hints: bool,
) -> Vec<String> {
    if session_empty {
        return crate::obvious_bubble::bubble_send_text(first_lifecycle)
            .into_iter()
            .collect();
    }
    if !agent_input_hints {
        return Vec::new();
    }
    oneshot_replies
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
```

`first_lifecycle` is the same source as today’s soft oneshot hint:
`obvious_chrome.lifecycle.first()` (already empty-send `/ds-…` form) or
`scope_facts.next_command` when needed for non-chrome scopes — prefer lifecycle\[0\] when
chrome was composed, since empty-session chrome always includes it when a lifecycle option
exists.

**Readiness:** empty-session disk seed is always **ready** (never sets
`default_prompts_pending`). Loading strip remains only for in-flight oneshot when agent
hints are on. `empty_submit_text` / `can_cycle_defaults` / `defaults_chrome` keep their
signatures; they receive the **effective** list length and pending flag from callers.

Update unit tests that assert “failed oneshot + heuristic → empty list”: still true for
**non-empty** sessions. Add scenarios for empty session + lifecycle seed, and for agent
flag off.

Keep `heuristic_as_prompts` as a thin helper if useful for tests; production list building
goes through `effective_prompts` only.

## Oneshot launch gate

In `main`’s `TurnComplete` path (today always starts a reply oneshot when assistant text
exists):

```rust
// crates/duckboard/src/main.rs  (TurnComplete, non-priming)

if state.config.chat.agent_input_hints
    && !was_priming
    && let Some(handle) = ax.agent_handle.clone()
    && let Some((assistant, user)) =
        default_prompts::last_assistant_and_user(&ax.session)
{
    ax.begin_default_prompts_oneshot();
    // … existing soft heuristic + task spawn
}
```

When agent hints are off, do not bump gen / pending and do not clear a disk seed that only
exists via pure recompute (oneshot storage may already be empty). Soft heuristic and
request framing stay unchanged when the oneshot **does** run.

## Auto messages gate

Extend pure visibility and “when visible” key helpers so a single flag darkens chips and
unbinds chrome hotkeys.

```rust
// crates/duckboard/src/obvious_bubble.rs

pub fn chrome_visible(
    is_streaming: bool,
    input_empty: bool,
    chrome: &ObviousChrome,
    auto_messages: bool,
) -> bool {
    auto_messages && !is_streaming && input_empty && !chrome_is_empty(chrome)
}

// resolve_cmd_*_when_visible: pass auto_messages through to chrome_visible
```

`build_obvious_chrome` / `refresh_obvious_chrome` stay composition-only (always refresh
the value for soft hint + empty-session seed). Gating is presentation/activation only —
turning auto messages back on does not require recomposing from disk.

`SendObviousAction` already re-checks `chrome_visible`; update that call site with the
flag so stale clicks are no-ops when the setting is off.

## Wire-up

Centralize list computation so view, Enter, and Tab agree.

```rust
// crates/duckboard/src/area/interaction.rs  (sketch)

fn session_input_hints(
    ax: &AgentSession,
    agent_input_hints: bool,
) -> Vec<String> {
    let session_empty = ax.session.messages.is_empty();
    let first = ax.obvious_chrome.lifecycle.first().map(String::as_str);
    crate::default_prompts::effective_prompts(
        session_empty,
        first,
        &ax.agent_default_prompts,
        agent_input_hints,
    )
}
```

Call sites:

```
| Site | Change |
|------|--------|
| `view_column` → `agent_chat::view` | Pass `session_input_hints(...)`; pending only when agent path pending |
| `SendPressed` empty branch | Same effective list + existing `empty_submit_text` |
| `CycleDefaultPrompt` | Same list + `can_cycle_defaults` |
| Settings toggle agent off | Clear oneshot storage / pending on sessions (or next view ignores oneshot because flag is false — prefer pure ignore; optional clear for cleanliness) |
| Chrome / key path | Pass `config.chat.auto_messages` into `chrome_visible` and resolvers |
```

`view_column` / chat key handling need access to `Config` (or the two bools). Prefer
threading `agent_input_hints` and `auto_messages` as parameters from `main`/area view
callers that already hold `state.config`, rather than reading config inside pure helpers.

Empty session + first lifecycle: list length 1 → Tab cycle is a no-op wrap; Enter sends
that entry. Same `↳` row UI as agent hints.

When the first user message lands, `messages` is non-empty → effective list drops the disk
seed immediately (even before an assistant reply). Agent list only appears after a later
oneshot if agent hints are on.

## Decisions

- **Empty-session seed is pure recompute, not stored in `agent_default_prompts`.**
  Alternatives: write lifecycle into `agent_default_prompts` on refresh (rejected: stale
  after first send; confuses “agent” storage with disk). Pure function keeps one source of
  truth for emptiness.

- **Agent flag only; disk seed not toggled.** Matches product choice C. Auto messages is a
  separate flag for chips only — under-input empty-session seed stays available when chips
  are dark.

- **Gate chrome at visibility, not composition.** Alternatives: skip
  `build_obvious_chrome` when off (rejected: soft oneshot hint and empty-session seed
  still need lifecycle\[0\]).

- **Keep capability paths and field names.** Docs/Settings say input hints / auto
  messages; `chat/default-prompts`, `chat/obvious-bubble`, and `agent_default_prompts`
  stay. Avoids backlink and rename churn.

- **Config nested under `[chat]`.** Alternatives: flat top-level keys (rejected: groups
  chat affordances for future flags).

## Risks

- **Existing configs without `[chat]`** → serde defaults apply agent off / auto messages
  on; intentional product defaults (agent suggestions become opt-in for everyone).

- **Redundant UI on empty session** (under-input seed + chips both show first lifecycle
  when auto messages on) → accepted; chips still expose multi-option and gates, list gives
  Enter parity with agent hints.

- **First message clears disk seed before any assistant reply** → brief empty under-input
  until oneshot (if enabled). Matches “no messages at all” rule; no special “waiting for
  first assistant” state.

## Open questions

None.
