# Agent default prompts — Design

Add a Provider oneshot sibling of `title_summary` that returns 0–3 suggested user replies;
duckboard merges them with `obvious_command` (always last, deduped) and drives empty-input
multi-option Enter with Tab cycling.

## Approach

Mirror the existing title-summary pipeline: fire a cheap-model oneshot on the session’s
harness after a turn, keep pure merge logic in duckboard, store results only in ephemeral
session state. Do not change `change_scope_facts` or session orientation.

```
TurnComplete (not priming)
        │
        ├─► title_summary          (existing — first real turn only)
        │
        └─► reply_suggestions      (new — every non-priming turn)
                 │
                 │  inputs:
                 │    last assistant text
                 │    last user text (when present)
                 │    available slash names (skill-first priming)
                 │
                 │  cheap model, same harness, no tools, no resume
                 ▼
            Vec<String>            // 0–3 parsed REPLY lines
                 │
                 ▼
         merge_default_prompts(agent, obvious_command)
                 │  agent order first
                 │  heuristic always last if unique
                 │  key-dedupe: /ds-spec ≡ ds-spec
                 ▼
         empty composer
                 │  show full list
                 │  Tab / Shift-Tab → active_idx
                 │  Enter → send active (or heuristic-only list)
```

While a new turn is streaming, agent suggestions clear immediately so the user never sends
a stale reply against a new answer. The lifecycle heuristic remains available as soon as
`obvious_command` is set (independent of the oneshot).

## Provider oneshot

Extend `duckchat`’s `Provider` trait with a second short-lived call, parallel to
`title_summary`. Both harnesses reuse their existing cheap-model picker
(`claude-haiku-4-5` / `grok-composer-2.5-fast`, with the same fallback-to-any-advertised
rule).

```rust
// crates/duckchat/src/request.rs

/// Input to `Provider::reply_suggestions`. Conversation-local only — never carries the
/// disk lifecycle heuristic (caller appends that after parse).
pub struct ReplySuggestionRequest {
    /// Latest assistant message text (required). Empty → empty suggestions.
    pub assistant_message: String,
    /// Immediately preceding user message text, when present.
    pub user_message: Option<String>,
    /// Slash command names the project exposes (e.g. `ds-spec`). Used only to prime
    /// skill-first wording; not a hard allow-list — unknown `/…` from the model is kept.
    pub available_commands: Vec<String>,
}

// crates/duckchat/src/provider.rs  (trait addition)
async fn reply_suggestions(
    &self,
    req: ReplySuggestionRequest,
    working_dir: &Path,
) -> Result<Vec<String>, Error>;
```

**Prompt framing** (shared intent; each harness embeds it the way it embeds the title
instruction — system channel for Claude, inline preamble for grok):

1. You are a reply-suggestion tool. Output only `REPLY:` lines.

2. Prefer skill/stage calls when the assistant is steering workflow (run `/ds-*`, skip a
   stage, choose between stages after review).

3. Prefer short user-voice replies when the assistant asks for confirmation or a natural
   choice.

4. Emit 1–3 lines of the form `REPLY: <text>`. No preamble, no quotes, no tools.

5. Available commands (hints): `…` (from `available_commands`).

**Parse** (shared pure helper, e.g. `duckchat::reply_suggest::parse_replies`):

```rust
// crates/duckchat/src/reply_suggest.rs  (or under each harness + re-export one parse)

/// Lines starting with `REPLY:` (case-sensitive prefix), trimmed after the colon.
/// Empty lines dropped; hard cap 3; order preserved.
pub fn parse_replies(raw: &str) -> Vec<String> { todo!() }
```

Implementations:

```rust
// ClaudeCodeProvider / GrokProvider
async fn reply_suggestions(...) -> Result<Vec<String>, Error> {
    // pick cheap model (same helper as title)
    // one-shot prompt, collect assistant text only
    // Ok(parse_replies(&raw))
    todo!()
}
```

Empty `assistant_message` short-circuits to `Ok(vec![])` without a model call.

## Merge and keys

Pure functions in duckboard (unit-testable; seed the capability scenarios). Heuristic is
**never** an input to the oneshot — only to merge.

```rust
// crates/duckboard/src/default_prompts.rs  (new)

/// Canonical equality key: trim, strip one leading `/`, ascii-lowercase.
pub fn prompt_key(s: &str) -> String { todo!() }

/// Agent replies first (order preserved, internal dedupe by key), then `heuristic`
/// as last entry when present and not already represented.
/// `heuristic` is the raw command without slash (today's `obvious_command`); when
/// appended it is formatted as `/{cmd}` so Enter sends a slash command.
pub fn merge_default_prompts(
    agent_replies: &[String],
    heuristic: Option<&str>,
) -> Vec<String> { todo!() }

/// Effective list for the empty composer: `merge_default_prompts` over current state.
pub fn effective_prompts(
    agent_replies: &[String],
    obvious_command: Option<&str>,
) -> Vec<String> { todo!() }
```

Examples:

```
| agent | heuristic | result |
|-------|-----------|--------|
| `/ds-spec` | `ds-design` | `[/ds-spec, /ds-design]` |
| `/ds-spec`, `/ds-step` | `ds-apply` | `[/ds-spec, /ds-step, /ds-apply]` |
| `yes, go ahead` | `ds-apply` | `[yes, go ahead, /ds-apply]` |
| `/ds-step` | `ds-step` | `[/ds-step]` |
| _(empty / not ready)_ | `ds-design` | `[/ds-design]` |
```

## Session state and dispatch

Ephemeral fields on `AgentSession` — not persisted in `chat_store` (proposal: no
cross-restart persistence).

```rust
// crates/duckboard/src/area/interaction.rs  — AgentSession additions

/// Agent-sourced default prompts from the latest completed turn (pre-merge).
pub agent_default_prompts: Vec<String>,
/// Monotonic generation; oneshot results apply only when gen matches.
pub default_prompts_gen: u64,
/// Index into `effective_prompts(...)` for Tab cycle / Enter. Clamped on list change.
pub default_prompt_idx: usize,
```

**When to fire:** every non-priming `TurnComplete` (same gate as “real turn ended,” not
limited to first turn like titles). On `TurnComplete`:

1. Bump `default_prompts_gen`, clear `agent_default_prompts`, reset `default_prompt_idx`.

2. Extract last assistant text and optional last preceding user text from
   `ax.session.messages` (skip priming messages).

3. Collect `available_commands` from `ax.chat_commands` names.

4. Spawn `Task` → harness `reply_suggestions` →
   `Message::DefaultPromptsReady { key, gen, result }`.

**On ready:** if `gen == ax.default_prompts_gen`, store parsed list (already capped);
clamp `default_prompt_idx`. On error or empty: leave `agent_default_prompts` empty
(heuristic-only list via merge).

**On new user send / streaming start:** bump gen and clear agent prompts so a late oneshot
cannot repopulate after the user has moved on. Heuristic (`obvious_command`) is unchanged
by that clear.

```rust
// crates/duckboard/src/main.rs  (sketch)

Message::DefaultPromptsReady { key, gen, result } => {
    // locate session by key; if gen mismatch, ignore
    // Ok(list) → ax.agent_default_prompts = list; clamp idx
    // Err(_) → warn; leave empty
}
```

Dispatch reuses the title path’s harness match (`"grok"` vs claude-code) and
`handle.working_dir()`.

## Composer UI and keyboard

Replace the single-placeholder / single empty-Enter path with a list-driven path.

**Display** when input is empty (trimmed) and not streaming with a queue-only path that
already owns Enter:

```
┌──────────────────────────────────────────┐
│                                          │  ← empty TextEdit (no single-cmd placeholder)
├──────────────────────────────────────────┤
│ › /ds-spec                               │  ← active (default_prompt_idx)
│   /ds-design                             │  ← heuristic tail
│ Enter · Tab next · Shift-Tab prev        │  ← optional quiet hint
└──────────────────────────────────────────┘
```

- Ghost selection: cycling does **not** fill the editor buffer (user can still type
  freely).

- When input is non-empty, hide the list; normal typing/send applies.

- When the list is empty (no agent, no heuristic), Enter is a no-op as today.

**Enter** (`agent_chat::Msg::SendPressed`, empty non-streaming branch):

```rust
// was: obvious_command → Some(format!("/{c}"))
// now:
let prompts = effective_prompts(&ax.agent_default_prompts, ax.obvious_command.as_deref());
let typed_opt = if typed.is_empty() {
    prompts.get(ax.default_prompt_idx).cloned()
} else {
    Some(typed)
};
```

**Tab / Shift-Tab** in `handle_agent_chat_key`, **after** completion-popup handling:

```
completion popup visible     → Tab = accept completion (unchanged)
else input empty && !prompts.is_empty()
                             → Tab = next idx, Shift-Tab = prev (wrap)
else                         → NotHandled
```

```rust
// agent_chat::Msg additions
CycleDefaultPrompt(i8),  // +1 / -1
```

Placeholder text: either omit the old `Press Enter to run /{cmd}` single line, or show a
generic `Press Enter to send · Tab to cycle` when `prompts.len() > 1`. Exact chrome is
presentation detail under the capability’s behavioral rules.

## Decisions

- **No heuristic in the oneshot prompt** — model stays conversation-local; merge always
  appends the ladder. Alternatives: pass heuristic as a soft hint (rejected: biases the
  model toward the ladder and fights skip-design / post-review cases).

- **Fire every non-priming TurnComplete** — keeps defaults ready when the user looks at
  empty input. Alternatives: lazy on first empty paint (rejected: extra state and flash).

- **Include last user message** when present — improves post-`/ds-review` and multi-turn
  steers. Alternatives: assistant-only (rejected: thinner context for forks).

- **Ghost list, not fill-on-Tab** — Tab only moves `default_prompt_idx`; buffer stays
  empty until the user types or Enter sends. Alternatives: fill buffer on Tab (rejected:
  fights free typing and undo noise).

- **Unknown `/…` kept** — no allow-list filter on parse. Available commands are priming
  only. Alternatives: drop unknown slashes (rejected: product call).

- **Heuristic always last when unique** — including when agent only returned free-text.
  Alternatives: suppress heuristic when any agent skill is present (rejected: product
  call; escape hatch matters after review).

- **Ephemeral only** — not written to session JSON. Alternatives: persist (rejected: stale
  after reopen; cheap to recompute on next turn).

## Risks

- **Oneshot latency** → heuristic-only list until ready; no spinner required; gen race
  drops late results.

- **Noisy or empty model output** → `parse_replies` returns `[]` → heuristic-only; same as
  today for empty Enter.

- **Tab collision with slash completion** → completion popup always wins when visible;
  cycle only when input empty and popup hidden.

- **Cost per turn** → short prompt + tiny completion on the cheapest model; acceptable
  mirror of title_summary frequency (title is rarer; this is every turn — monitor if
  needed, but out of scope to rate-limit now).

## Open questions

None — last-user inclusion resolved: **include** when present.
