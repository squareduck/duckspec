# Next-card composer hints - Design

Align empty-composer next actions with trailing `next` meta cards (ghost + Tab), move
optional oneshot suggestions to a single under-input / ⇧↩ path with a freeform “good
reply” brief, retire lifecycle auto-message composition while keeping a generic option
chrome shell, drop the auto-messages setting, and tint meta-card lines in the transcript.

## Approach

```
assistant markdown (string)              empty session (0 turns)
        │                                        │
        ▼                                        ▼
  duckboard meta_card parser              lifecycle[0] bootstrap only
  (line scan; no duckpond)                       │
        │                                        │
        ├─ all write|next cards ──► line ranges ──► LineBgKind::MetaCard
        │
        └─ trailing next ──► NextAction[] (send tokens)
                    │
                    ▼
         next_actions + active_idx
                    │
         ┌──────────┴──────────────┐
         ▼                         ▼
   TextEdit placeholder      tab-available marker
   (ghost when empty)        (only if len > 1; same
         │                    language as hint marker)
         Enter empty → send
         Tab/⇧Tab    → cycle

oneshot (agent_input_hints ON)          obvious chrome
        │                                     │
        ▼                                     ▼
  full last user + last assistant       options[] + cancel?
  instruction: freeform good reply      not fed by lifecycle
  ≤1 REPLY under input                  (question wiring later)
  marker ⇧↩ · ⇧↩ sends or no-op
```

Authority split:

```
| Source               | When                       | UI                                 |
|----------------------|----------------------------|------------------------------------|
| Disk lifecycle `[0]` | `messages.is_empty()` only | ghost seed; no chips               |
| Trailing `next` card | ≥1 turn                    | ghost + Tab; send = backtick token |
| No trailing `next`   | ≥1 turn                    | no next ghost / cycle              |
| Oneshot              | `agent_input_hints` on     | under-input + ⇧↩ only              |
| Option chrome        | unpopulated this change    | shell retained                     |
```

## Meta-card parse (duckboard only)

New pure module `crates/duckboard/src/meta_card.rs`. **Do not call duckpond.**
Line-oriented scan of the assistant message string — chat chrome only; no shared types or
dependency on duckpond parsers for this feature.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaCardKind { Write, Next }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaCard {
    pub kind: MetaCardKind,
    pub line_start: usize, // 0-based inclusive
    pub line_end: usize,   // 0-based inclusive
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextAction {
    pub send: String,           // first `token` on the line
    pub reason: Option<String>, // remainder after token (label only)
}

pub fn parse_meta_cards(source: &str) -> Vec<MetaCard> { … }
pub fn trailing_next_actions(source: &str) -> Vec<NextAction> { … }
```

Recognition (aligned with `style`, implemented independently):

- A meta card is a maximal run of blockquote lines (`>` with optional space).

- First non-empty content in the run is exactly `**write**` or `**next**` (after trim).

- Card ends at the first non-blockquote line.

- Fence-aware: lines inside open fenced code blocks are not treated as quote lines (track
  fence open/close); keep the scanner simple and unit-tested.

- Action lines (`next` body): first inline code span → `send`; rest trimmed → `reason`.
  Cap at 3. Skip lines without a token.

- **Trailing next for actions:** last `Next` card whose `line_end` is at the end of the
  source (only blank lines may follow).

- **Tint:** every line in any `Write` or `Next` card → `LineBgKind::MetaCard`.

## Next-action composer

Re-home the primary empty-input list away from oneshot.

```rust
// AgentSession (conceptual)
next_actions: Vec<NextAction>,
next_action_idx: usize,
agent_default_prompts: Vec<String>, // ≤1 after settle
default_prompts_pending: bool,
// no cycle index for oneshot (single suggestion)
```

```rust
pub fn next_action_list(
    session_empty: bool,
    bootstrap: Option<&str>,      // lifecycle[0] empty-send form
    last_assistant: Option<&str>, // non-priming assistant plain text
) -> Vec<NextAction> { … }
```

- Empty session: 0 or 1 bootstrap action from `scope_facts` / lifecycle first command
  (empty-send form with leading `/` when needed).

- Non-empty session: `trailing_next_actions(last_assistant)` only — never disk fallback,
  never oneshot.

- Refresh on turn complete, session load, and when the last assistant message changes.

Empty-composer bindings:

```
| Input empty | Binding    | Behavior                                     |
|-------------|------------|----------------------------------------------|
| yes         | Enter      | send `next_actions[idx]` if any              |
| yes         | Tab / ⇧Tab | cycle next actions if `len > 1`              |
| yes         | ⇧Enter     | send oneshot suggestion if armed; else no-op |
| no          | ⇧Enter     | existing newline (`TextEdit`)                |
| yes         | ⌘Enter     | no-op for hints (no affirm dual-purpose)     |
```

Markers (same visual family as today’s under-input `↳`):

```
| Surface                | When                       | Marker                                     |
|------------------------|----------------------------|--------------------------------------------|
| Next multi             | `next_actions.len() > 1`   | small tab-available indicator (e.g. `⇥`)   |
| Under-input agent hint | oneshot ready, single line | `⇧↩` before suggestion text (replaces `↳`) |
```

- Ghost: `TextEdit::placeholder(active.send)` when input empty and a next action is armed.

- Under-input column lists **only** oneshot chrome (loading or one suggestion) — never
  next-card actions.

## Agent oneshot (role + framing)

**Product role:** freeform suggestion of a natural user reply given the last user message
and the last assistant message — not lifecycle / stage autocomplete (that is the `next`
meta card).

```rust
// duckchat
pub const MAX_REPLIES: usize = 1;

// REPLY_SUGGEST_INSTRUCTION — rewrite:
// - Output 0–1 REPLY: lines only
// - Suggest a natural user response continuing the dialogue
// - Do not prefer /ds-* stage commands as the default job
// - No lifecycle_heuristic soft-hint framing

// build_reply_suggest_prompt:
// - Pass full user_message and assistant_message (no line truncation)
// - Drop lifecycle_heuristic from prompt body and from ReplySuggestionRequest
// - Prefer drop available_commands (avoids slash-command steering)
```

Remove `ASSISTANT_PROMPT_MAX_LINES` / `USER_PROMPT_MAX_LINES`, `take_last_lines` (if
unused), and related truncation tests.

Launch gate:

```rust
should_begin_reply_oneshot(agent_input_hints, was_priming, has_assistant)
// no auto_messages term
```

Empty ⇧↩: if `agent_input_hints` and oneshot ready with a non-empty string, send it;
otherwise no-op for this purpose.

## Obvious chrome shell

Collapse lifecycle chrome to a question-ready core; stop composing phase options on the UI
path.

```rust
pub struct ObviousChrome {
    /// Ordered options (send text). Empty ⇒ chrome hidden.
    pub options: Vec<String>,
    /// When set, ⌘⌫ sends this (later: cancel question).
    pub cancel: Option<String>,
}
```

- Keep `resolve_cmd_digit`, `resolve_cmd_backspace`, `chrome_visible`, and chip view
  behind non-empty options (for a later question change).

- Remove affirm dual-⌘↩, lifecycle dual-enter presentation, and `build_obvious_chrome`
  phase-table use from the hot path.

- `refresh_obvious_chrome`: leave chrome empty; still refresh `scope_facts` for first-turn
  orientation and empty-session bootstrap only.

## Config

```toml
[chat]
agent_input_hints = false   # under-input oneshot; ⇧↩ to send
```

- **Remove** `ChatConfig.auto_messages` and the settings toggle entirely.

- Ensure existing `config.toml` files that still list `auto_messages` load without failure
  (ignore unknown keys or match existing config unknown-field policy).

- All call sites that threaded `auto_messages` into `effective_prompts` /
  `should_begin_reply_oneshot` / `chrome_visible` lose that parameter; chrome is simply
  empty, so visible only when a future path fills `options`.

## Transcript tint

When syncing Answer `EditorState` lines, set
`line_backgrounds[i] = Some(LineBgKind::MetaCard)` for every line covered by any parsed
meta card. Theme: quiet band, distinct from search `Match` and diff hunks.

## Impact

```
| Area                                | Effect                                           |
|-------------------------------------|--------------------------------------------------|
| `crates/duckboard/src/meta_card.rs` | New line parser + unit tests                     |
| `default_prompts` / `AgentSession`  | Next list vs oneshot split; no auto_messages     |
| `agent_chat` composer               | Ghost, tab marker, ⇧↩ marker, no next under-list |
| `text_edit`                         | `LineBgKind::MetaCard`; placeholder ghost        |
| `obvious_bubble`                    | Generic shell; no phase composition in UI        |
| `config` / settings                 | Drop `auto_messages`; keep `agent_input_hints`   |
| `duckchat` `reply_suggest`          | New instruction; max 1; full context; no heur.   |
| Caps                                | Rewrite default-prompts + obvious-bubble; tint   |
|                                     | in transcript                                    |
| Templates                           | No syntax change; agents must emit trailing next |
| Persistence                         | No session schema change; next_actions ephemeral |
```

**Breakage:** users who relied on lifecycle chips (former default-on auto messages) lose
that path; empty Enter after the first turn requires a trailing `next` meta card from the
agent.

## Decisions

- **Zero duckpond coupling for meta cards** — independent line parser in duckboard only.
  Alternative: reuse duckpond `elements` (rejected: ties chat chrome to artifact Layer 1).

- **⇧↩ for oneshot, Enter for next ghost** — empty-input ⇧↩ does not insert a newline;
  non-empty keeps newline.

- **Tab-available marker only when multi next** — same “key before affordance” pattern as
  the old `↳` / new `⇧↩` oneshot marker; no full next-action list under the input.

- **Oneshot is freeform reply suggestion** — rewrite instruction; strip lifecycle
  autocomplete, lifecycle_heuristic, and context line truncation.

- **Send exact backtick tokens** from the card (`confirm`, `/ds-propose`, …).

- **Drop `auto_messages` from config and settings** — chrome population is gone; a dead
  toggle is worse than deletion.

- **Chrome shell kept empty** for a later structured-question change.

## Risks

- **Agents omit trailing `next`** → empty ghost after turns. Mitigation: template
  discipline (stages already emit cards at gates/handoffs); no UI invent.

- **False-positive quote runs** → fence-aware line scanner + unit tests.

- **Full-context oneshot cost** → acceptable; `agent_input_hints` stays default off.

- **⇧↩ discoverability** → under-input `⇧↩` marker (same family as old `↳`).

- **Old config.toml still has `auto_messages`** → ensure load does not fail; ignore
  unknown key or strip on read per existing config behavior.
