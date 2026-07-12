# Oneshot hint chips - Design

Fold freeform reply-suggestion oneshot into the existing fast-response shell, drop
under-input oneshot chrome, and delete the dead obvious-bubble surface with a rename pass
on leftover identifiers.

## Approach

Three empty-composer surfaces stay separate by **authority**, not by chrome style:

```
| Authority        | Chrome              | Activation                         |
|------------------|---------------------|------------------------------------|
| Next actions     | Input ghost         | Enter / Tab                        |
| Oneshot replies  | Fast-response chips | ⌘n / click → `send_prompt_text`    |
| Question tool    | Fast-response chips | ⌘n / click → `answer_user_choice`  |
```

```
TurnComplete
    │
    ├─ refresh_next_actions(after_turn=true)
    ├─ clear_user_choice_shell (existing)
    └─ maybe begin reply oneshot
            │
            │  only if agent_input_hints
            │  && !priming
            │  && has assistant text
            │  && next_actions.is_empty()   ← skip model when ghost would win
            ▼
       settle → agent_default_prompts (0–3)
            │
            └─ sync_oneshot_chips(ax)
                    │
                    ├─ awaiting user?     → leave UserChoice shell alone
                    ├─ next_actions ≠ []  → shell empty (no oneshot fill)
                    ├─ streaming?         → shell empty
                    ├─ prompts empty?     → shell empty
                    └─ else               → FastResponse { OneshotHints, options }
```

Priority (single shell, last writer must respect rules):

```
UserChoiceRequest  ──always──▶  source=UserChoice   (wins over oneshot)
oneshot settle     ──if idle, no ghost──▶  source=OneshotHints
refresh / turn start ──clear oneshot list──▶  re-sync (empty if no list)
```

Under-input path (`DefaultsChrome`, loading row, `SendOneshotSuggestion`, empty Cmd-Enter)
is deleted. No oneshot loading chrome.

## Reply oneshot (duckchat)

`crates/duckchat/src/reply_suggest.rs`:

```rust
pub const MAX_REPLIES: usize = 3;

// REPLY_SUGGEST_INSTRUCTION asks for up to three REPLY: lines in order:
// 1 most likely · 2 alternative · 3 negative/decline
// Omit a line if it does not fit. Same prefix for all lines.
```

Parser unchanged except the cap: ordered `REPLY:` lines, trim, drop empties, keep first 3.
Partial lists are valid product output.

## Default prompts helpers

`crates/duckboard/src/default_prompts.rs`:

- Keep next-action ghost helpers as-is

- `oneshot_display_prompts`: truncate to 3 (not 1); still gated by `agent_input_hints`

- Remove (or stop using): `DefaultsChrome`, `defaults_chrome`, `oneshot_cmd_submit_text`,
  `ONESHOT_CMD_ENTER_MARKER`

- Add pure gate for fill:

```rust
/// Whether settled oneshot replies may occupy the fast-response shell.
pub fn oneshot_chips_allowed(
    is_streaming: bool,
    is_awaiting_user: bool,
    next_actions_len: usize,
    agent_input_hints: bool,
    oneshot_len: usize,
) -> bool {
    agent_input_hints
        && !is_streaming
        && !is_awaiting_user
        && next_actions_len == 0
        && oneshot_len > 0
}
```

Launch gate: extend `should_begin_reply_oneshot` (or a thin wrapper at the call site) so
oneshot is **not started** when `next_actions` is non-empty after the turn refresh —
avoids paying for suggestions that can never show.

## Fast-response shell

`crates/duckboard/src/fast_response.rs`:

```rust
pub enum FastResponseSource {
    None,
    UserChoice { correlation_id: u64 },
    /// Settled freeform reply suggestions; activation sends a normal user turn.
    OneshotHints,
}

pub fn from_oneshot_hints(replies: impl IntoIterator<Item = String>) -> FastResponse {
    // id == label == reply text; take ≤9 (oneshot already ≤3)
}

// Rename bubble_send_text → lifecycle_send_text (or format_lifecycle_send)
pub fn lifecycle_send_text(command: Option<&str>) -> Option<String> {
    command.and_then(format_lifecycle_command)
}
```

Existing `visible` already: streaming without awaiting → hide; idle empty composer → show.
Oneshot never sets `is_awaiting_user`, so oneshot chips only appear when idle (proposal:
non-streaming). Question path unchanged.

## Session sync and activation

`AgentSession` still holds `agent_default_prompts` + gen/pending as the settled list
(ephemeral). Pending stays for supersession only — UI never shows loading.

```rust
// interaction.rs
pub fn sync_oneshot_chips(ax: &mut AgentSession, agent_input_hints: bool) {
    // if UserChoice parked: return without touching shell
    // else if oneshot_chips_allowed(...): fast_response = from_oneshot_hints(...)
    // else if source is OneshotHints or None: clear shell (do not clear UserChoice)
}

// activate_fast_response match arm:
FastResponseSource::OneshotHints => {
    // take option id/label as send text
    // clear shell + oneshot list
    // send_prompt_text(ax, text, highlighter)
}
```

Call `sync_oneshot_chips` from:

```
| Event | Behavior |
|-------|----------|
| `DefaultPromptsReady` (gen match) | store list, pending=false, sync |
| `TurnComplete` after `refresh_next_actions` | shell already cleared of UserChoice; oneshot not yet ready → sync empty |
| `refresh_fast_response` | today always `build_fast_response` empty when not awaiting — **must preserve** `UserChoice` (already) and either re-sync oneshot from `agent_default_prompts` or preserve `OneshotHints` |
| `send_prompt_text` / clear oneshot | clear list + sync empty |
| `UserChoiceRequest` | `from_user_choice` (overwrite any oneshot fill) |
```

Recommended: `refresh_fast_response` stops blindly assigning empty shell when not
awaiting; instead `sync_oneshot_chips` (or: if awaiting keep; else rebuild
oneshot-or-empty from list). Product path no longer uses disk lifecycle to fill chips.

## Composer UI

`widget/agent_chat.rs`:

- Remove under-input oneshot list/loading and `on_empty_cmd_submit(SendOneshotSuggestion)`
  (or make empty Cmd-Enter a no-op)

- Chips already render from `fast_response` when `visible`

- Ghost path unchanged

## Obvious-bubble deletion and renames

Delete `duckspec/caps/chat/obvious-bubble/` entirely (all scenarios already skipped; no
live `@spec` backlinks in code).

Rename pass (same change):

```
| Today | Rename to |
|-------|-----------|
| `bubble_send_text` | `lifecycle_send_text` |
| `compute_obvious_command` (test) | `compute_lifecycle_command` |
| `obvious_command_from_artifacts` (test) | `lifecycle_command_from_artifacts` |
| test names `obvious_*` | `lifecycle_*` |
| comments `refresh_obvious_command` / “obvious-command hint” | lifecycle / next-command language |
```

Keep `ChangeScopeFacts.next_command`, orientation, and empty-session ghost bootstrap —
only names that still say “bubble/obvious” for non-bubble roles.

## Caps impact

```
| Cap | Change |
|-----|--------|
| `chat/default-prompts` | 0–3 `REPLY:`; no under-input chrome; chips only when no ghost / idle; launch skip when ghost; partial OK; no loading |
| `chat/fast-response` | `OneshotHints` source; activation sends user message; refresh must not wipe oneshot incorrectly; priority vs UserChoice |
| `chat/obvious-bubble` | **delete** |
```

## Impact

- `duckchat` public const `MAX_REPLIES` and instruction string change (harnesses share
  framing)

- Duckboard message `SendOneshotSuggestion` removable

- Config `agent_input_hints` stays default off

- No session persistence change (oneshot remains ephemeral)

- Archive history may still mention the old cap; live tree does not

## Decisions

- **Skip oneshot when next_actions non-empty** — not only hide chips, but do not call the
  model. Alternative: always call, only hide (rejected: pure cost waste when ghost wins).

- **Replies stay in `agent_default_prompts`; shell is derived** — refresh/rebuild can
  re-sync without a second store. Alternative: oneshot only lives in `FastResponse`
  (rejected: refresh currently clears shell and would need special-case storage anyway).

- **id = label = reply text** for oneshot options — freeform send needs no separate wire
  id. Alternative: synthetic ids (rejected: extra map for no benefit).

- **Ghost gate = `next_actions` non-empty**, not “ghost string currently painted” — same
  authority as empty Enter, including while streaming would hide ghost text.

## Risks

- **`refresh_fast_response` wiping oneshot chips** → re-sync from `agent_default_prompts`
  whenever not awaiting UserChoice; tests for settle → refresh → still visible.

- **UserChoice then late oneshot settle overwriting chips** → settle path no-ops fill when
  `is_awaiting_user`; gen clear on turn start already limits stale settles.

- **Long freeform chip labels** → existing chip layout soft-wraps / truncates as today; no
  new layout work (proposal non-goal).
