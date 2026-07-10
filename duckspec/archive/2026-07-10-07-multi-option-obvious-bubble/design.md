# Multi-option obvious chrome — Design

Expand the single lifecycle obvious bubble into multi-option action chips (lifecycle
`/ds-*`, affirm, decline) with key-first labels and dual-purpose ⌘↩, driven only by disk
phase, session emptiness, and VCS dirty state.

## Approach

```
disk (change_scope_facts / archived?)
session (messages empty?)
vcs (changed_files dirty?)
        │
        ▼
build_obvious_chrome(...)     // pure; phase table from proposal
        │
        ▼
AgentSession.obvious_chrome   // replaces obvious_command
        │
        ├── soft hint ──▶ oneshot  (lifecycle[0] only; default-prompts unchanged)
        │
        ▼
chrome_visible? (idle, empty input, non-empty chrome)
        │
        yes ──▶ chips: [⌘n  /ds-…]  [⌘↩ Confirm] [⌘⌫ Reject]
        │              │
        │              ▼
        │         send_prompt_text(action string only)
        │
        no ──▶ no chrome; those keys unbound for this path
```

Boundaries:

- **In:** `obvious_bubble` pure helpers, change-area refresh, agent chat chrome,
  chat-focused key path when composer empty.

- **Out:** oneshot `REPLY:` list / empty Enter / Tab (`default-prompts`); agent handoff
  parsing; session FSM labels.

- **Orientation:** `ChangeScopeFacts.next_command` stays a single string = `lifecycle[0]`
  without the leading slash (unchanged contract for `session/scope`).

Chips are not faux user bubbles. Layout is hotkey first, then action text.

## ObviousChrome pure model

Module: `crates/duckboard/src/obvious_bubble.rs` (keep path; capability name unchanged).
Replace single-command helpers with a chrome value and key resolution.

```rust
// crates/duckboard/src/obvious_bubble.rs

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObviousChrome {
    /// Ordered lifecycle actions in empty-send form (`/ds-step`, …).
    pub lifecycle: Vec<String>,
    /// Affirm row: Confirm (gate) or Commit (post-archive dirty).
    pub affirm: Option<Affirm>,
    /// When true, show Reject and bind ⌘⌫.
    pub decline: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Affirm {
    Confirm,
    Commit,
}

impl Affirm {
    pub fn send_text(self) -> &'static str {
        match self {
            Affirm::Confirm => "Confirm",
            Affirm::Commit => "Commit",
        }
    }
}

/// Empty-send form: bare `ds-foo` → `/ds-foo`; already-slashed preserved.
pub fn format_lifecycle_command(cmd: &str) -> Option<String> { todo!() }

pub fn chrome_is_empty(chrome: &ObviousChrome) -> bool {
    chrome.lifecycle.is_empty() && chrome.affirm.is_none() && !chrome.decline
}

/// Idle + empty composer + non-empty chrome. Oneshot pending is not a gate.
pub fn chrome_visible(
    is_streaming: bool,
    input_empty: bool,
    chrome: &ObviousChrome,
) -> bool {
    !is_streaming && input_empty && !chrome_is_empty(chrome)
}

/// ⌘↩ target: affirm send text if present, else lifecycle[0], else None.
pub fn resolve_cmd_enter(chrome: &ObviousChrome) -> Option<String> { todo!() }

/// ⌘⌫ target: Some("Reject") only when decline is true.
pub fn resolve_cmd_backspace(chrome: &ObviousChrome) -> Option<String> { todo!() }

/// ⌘1…⌘9: 1-based index into lifecycle; out of range → None.
pub fn resolve_cmd_digit(chrome: &ObviousChrome, digit: u8) -> Option<String> {
    todo!()
}

/// Chip label: hotkey glyph then action, e.g. "⌘1  /ds-step", "⌘↩  Confirm".
pub fn lifecycle_chip_label(index_1based: usize, action: &str) -> String { todo!() }
pub fn affirm_chip_label(affirm: Affirm) -> String { todo!() }
pub fn decline_chip_label() -> String { todo!() }
```

Activation always sends the **action string only** (never the hotkey prefix). Click on a
chip uses the same send path as the matching key.

Unit tests stay in this module (visibility, resolve order, format, labels).

## Phase builder and refresh

Extend `change_scope_facts` so multi-option ranks are available without re-deriving phase
ad hoc. Prefer adding `lifecycle_commands: Vec<String>` (bare names, no slash) and keep
`next_command` as `lifecycle_commands.first()` for orientation / soft hint.

```rust
// crates/duckboard/src/area/change.rs  (sketch)

pub struct ChangeScopeFacts {
    pub phase: &'static str,
    pub steps_done: usize,
    pub step_count: usize,
    pub active_step_tasks: Option<(usize, usize)>,
    /// lifecycle[0] bare name — orientation + oneshot soft hint.
    pub next_command: Option<String>,
    /// Full ordered bare names for this phase (may be length 1).
    pub lifecycle_commands: Vec<String>,
    pub current_review: Option<String>,
}

/// Pure chrome for one scope + session + dirty flag.
pub fn build_obvious_chrome(
    scope: &crate::scope::Scope,
    project: &ProjectData,
    session_empty: bool,
    vcs_dirty: bool,
) -> ObviousChrome {
    todo!()
}

/// Refresh every interaction session's `obvious_chrome`.
pub fn refresh_obvious_chrome(
    interactions: &mut HashMap<Scope, InteractionState>,
    project: &ProjectData,
    vcs_dirty: bool,
) {
    todo!()
}
```

Phase → lifecycle bare names (proposal contract):

```
exploration + session_empty     → [ds-explore]
exploration + nonempty          → []
empty change                    → [ds-propose]
proposal, no design, no caps    → [ds-design, ds-spec]
design, no caps                 → [ds-spec]
caps, no steps                  → [ds-step, ds-spec, ds-archive]
open steps                      → [ds-apply, ds-review]
all steps done                  → [ds-archive, ds-review]
archived                        → [] from facts; chrome may still get Commit
```

Gate / affirm / decline (applied after lifecycle list):

```
if Scope::Change && !session_empty && not Commit-only path:
  affirm = Confirm, decline = true

if Scope::Change(name) in archived
   && !session_empty
   && vcs_dirty:
  lifecycle = []
  affirm = Commit
  decline = false

Exploration / Caps / Codex: no Confirm/Reject pair
Caps / Codex: empty chrome (unchanged)
```

`refresh_obvious_chrome` needs `session_empty` per `AgentSession` (from
`session.messages.is_empty()`) and a single `vcs_dirty` from `!changed_files.is_empty()`
(or equivalent already on change area state). Call sites that today call
`refresh_obvious_command` switch to the new refresh; also re-run when messages go
empty↔nonempty and when VCS file list updates.

Soft hint for oneshot request construction:

```rust
let heuristic = chrome
    .lifecycle
    .first()
    .map(|s| s.trim_start_matches('/').to_string());
// or keep ChangeScopeFacts.next_command and pass that — same value
```

`default_prompts::heuristic_as_prompts` keeps formatting a single optional command via
`format_lifecycle_command`; no multi-option list in the composer.

## Session field

```rust
// crates/duckboard/src/area/interaction.rs

pub struct AgentSession {
    // …
    /// Multi-option obvious chrome. Ephemeral — refreshed, not persisted.
    pub obvious_chrome: crate::obvious_bubble::ObviousChrome,
    // remove: pub obvious_command: Option<String>,
}
```

Migration: grep-replace all `obvious_command` readers. Soft-hint call sites use
`scope_facts.next_command` or `obvious_chrome.lifecycle.first()`.

Key path (today ⌘↩ → `SendObvious`):

```rust
// resolve when chrome_visible
// ⌘↩        → Msg::SendObviousAction(text)  // from resolve_cmd_enter
// ⌘⌫        → same with resolve_cmd_backspace
// ⌘1…⌘9     → same with resolve_cmd_digit
// chip click → same Msg with that chip's send text
```

Prefer one message variant carrying `String` over separate enums so send stays one path
into `send_prompt_text`.

⌘ bindings only when `chrome_visible` (idle + empty input + non-empty chrome) so typing is
never stolen; ⌘⌫ does not fight line-delete while the composer has text.

## Chip UI

```
// crates/duckboard/src/widget/agent_chat.rs

// After transcript, above composer — view chrome only, not ChatSession.messages

for (i, action) in chrome.lifecycle.iter().enumerate() {
    chip(lifecycle_chip_label(i + 1, action), SendObviousAction(action.clone()))
}

// Gate row: horizontal pair when affirm is Confirm and decline
// or single Commit chip when affirm is Commit

[ ⌘↩  Confirm ] [ ⌘⌫  Reject ]
[ ⌘↩  Commit ]
```

Styling: muted action chips (reuse / adapt `theme::chat_obvious_bubble`), not user-bubble
paper. First lifecycle chip may show `⌘1 · ⌘↩` only when affirm is absent and that chip is
also the ⌘↩ target (optional polish; labels may stay `⌘1` only and rely on key
resolution).

## Spec surface

Modify capability `chat/obvious-bubble` via rewrite or large delta:

- Requirements for chrome contents (lifecycle ranks, gate rules, Commit)
- Visibility (same gates, non-empty chrome)
- Key resolution (⌘↩ / ⌘⌫ / ⌘n)
- Display format (hotkey first)
- Activation send (action string; not oneshot list)
- Ephemeral chrome (not stored until send)

`session/scope` unchanged unless tests assume a single field name; orientation text still
“Suggested next stage: /{next_command}”.

`chat/default-prompts` untouched.

## Decisions

- **Replace `obvious_command` with `ObviousChrome`** — one structured field on the
  session. Alternatives: keep `obvious_command` as lifecycle[0] plus side fields
  (rejected: split source of truth for visibility and keys).

- **Gate row = nonempty change session** — show Confirm+Reject without agent parse.
  Alternatives: phase-only gates (rejected: misses gates on “wrong” phase); always show
  including exploration (rejected: noise).

- **Commit is affirm-only** — no Reject post-archive. Alternatives: Reject beside Commit
  (rejected: no clear object).

- **Soft hint remains single lifecycle[0]** — oneshot request unchanged in shape.
  Alternatives: pass full lifecycle list as hint (rejected: out of scope for
  default-prompts this change).

- **Hotkey glyph `⌘`** — match existing `⌘↩` chrome; not the string `cmd-`.

- **Lifecycle ranks** — as frozen in the proposal (caps → step/spec/archive;
  all-steps-done → archive/review). Single `next_command` for orientation = first of those
  lists.

## Risks

- **⌘1…⌘9 collide with future global shortcuts** → bind only when chat-focused, chrome
  visible, composer empty; document in keybinds if needed.

- **Stray Confirm/Reject when agent is not at a write gate** → low cost (agent absorbs);
  missing Confirm is worse — keep bias to show gate on nonempty change sessions.

- **Repo-wide dirty triggers Commit after unrelated edits** → accept false positives; user
  ignores chip. Optional later: dirty only under change paths (out of scope).

- **Large rewrite of `obvious_bubble` unit tests / backlinks** → plan as one step with
  `@spec` updates; pure helpers keep tests local.

## Open questions

None material. Proposal freeze is the product contract; implement from this design and the
proposal tables.
