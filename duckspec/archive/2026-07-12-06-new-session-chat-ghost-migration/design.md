# New session chat ghost migration - Design

Carry the active session’s next-action list onto a newly created empty change chat
session, and keep it sticky under refresh until that session’s first turn.

## Approach

⌘N (`NewAction::NewChatSession`) and session **+** both emit
`interaction::Msg::NewSession`. Inheritance lives on that path only—no keybind- specific
branch.

```
Msg::NewSession (change multi-session)
  donor = ix.active()          # still the old active before insert
       │
       ├── next_actions empty ──▶ fresh AgentSession (bootstrap later)
       │
       └── next_actions non-empty
                │
                ▼
         fresh.inherited_next_actions = Some(clone)
         refresh_next_actions(true)   # list + idx 0
                │
                ▼
         insert at 0, active = 0

refresh_next_actions / refresh_fast_response (any tick)
  messages empty + inherited Some(non-empty)?
       yes ──▶ next_actions = inherited   (skip lifecycle bootstrap)
       no  ──▶ existing empty / non-empty rules
  messages non-empty ──▶ clear inherited; trailing `next` only
```

```
empty + inherited? ──yes──▶ inherited list
        │
        no
        ▼
empty + lifecycle[0] ──▶ bootstrap (unchanged)
        │
        no
        ▼
empty ──▶ []
non-empty ──▶ trailing next | []
```

Inheritance is **ephemeral** (like `next_actions` today)—not written to `chat_store`. App
restart of an empty session falls back to bootstrap.

## Inherited next-actions field

On `AgentSession` (`crates/duckboard/src/area/interaction.rs`):

```rust
/// Donor list for empty-session ghost continuity after NewSession.
/// Ephemeral — not persisted. Cleared when the session is no longer empty.
pub inherited_next_actions: Option<Vec<crate::meta_card::NextAction>>,
```

Seeded only when the donor’s `next_actions` is non-empty. `next_action_idx` starts at `0`
on the fresh session.

## Pure list resolution

Extend `default_prompts::next_action_list` so empty-session priority is inherited →
bootstrap → empty:

```rust
pub fn next_action_list(
    session_empty: bool,
    bootstrap: Option<&str>,
    last_assistant: Option<&str>,
    inherited: Option<&[NextAction]>,
) -> Vec<NextAction>
```

`AgentSession::refresh_next_actions`:

- Pass `self.inherited_next_actions.as_deref()` when empty

- When `!session_empty`, set `inherited_next_actions = None` before/after rebuild (first
  turn ends inheritance permanently)

Oneshot eligibility still keys off “is `next_actions` empty?”—no oneshot seed.

## NewSession seed helper

Shared helper so change and ideas multi-session paths stay aligned:

```rust
/// Capture active donor next_actions, build empty session, optionally inherit.
pub fn new_session_with_inherited_next_actions(
    ix: &InteractionState,
    scope_key: String,
    scope_kind: ScopeKind,
) -> AgentSession
```

Call sites (before `insert(0)` while `active_session` still names the donor):

- `crates/duckboard/src/area/change.rs` — `Msg::NewSession`
- `crates/duckboard/src/area/ideas.rs` — `Msg::NewSession if is_multi`

Scope: **change multi-session only** (proposal). `ideas` is already `Scope::Change`. On
the change area path, only seed inheritance when `scope_kind == ScopeKind::Change` (skip
pure exploration multi-session if it shares the handler).

⌘N needs no change: `main.rs` already dispatches the same `Msg::NewSession`.

## Cap doc / list authority

`caps/chat/default-prompts` empty-session source gains one rung above lifecycle bootstrap.
Non-empty session rules and “no disk fallback after first turn” unchanged. No new
capability path.

## Impact

- `AgentSession` + `refresh_next_actions` + `next_action_list` signature/tests
- Two `NewSession` handlers share one helper
- Spec/doc deltas on `chat/default-prompts` (list priority, NewSession continuity)
- No persistence schema, no harness, no keybind resolver changes

## Decisions

- **Sticky field, not one-shot assign** — `refresh_fast_response` would wipe a bare
  `next_actions` copy. Alternatives: skip refresh when empty (rejected: breaks
  scope_facts/bootstrap updates for sessions without inheritance); persist to disk
  (rejected: proposal non-goal).

- **Clone `NextAction` list only** — not oneshot chips, not transcript, not Tab index from
  donor. Alternatives: copy `next_action_idx` (rejected: clean chat starts at first ranked
  action per proposal).

- **Clear inheritance when non-empty** — first user message is enough; no need to hook
  send specially if every refresh path clears. Alternatives: clear only on `TurnComplete`
  (rejected: mid-first-turn refresh would still prefer inherited over empty trailing
  next).

- **Helper over duplicated copy in two handlers** — single seed path for + and area
  routing.

## Risks

- **Stale donor tokens** (e.g. `confirm` for a write gate that no longer applies) → accept
  per proposal; same as staying on the donor session

- **Donor mid-stream with partial list** → copy whatever `next_actions` is (often still
  the pre-turn list; ghost hidden while streaming on donor)

- **Forgot sticky field** → empty session silently reverts to bootstrap; unit tests on
  refresh with inherited + empty messages
