# Turn answer replace - Design

Four layers: replaceable answer draft, thrash-budget auto-cancel, shared write-gate
emit-once in style, Thinking body fade — all duckboard/session or shared chrome, not
provider-specific event coalesce.

## Approach

```
agent stream (any harness)
  ReasoningDelta / ContentDelta / ToolUse / …
        │
        ▼
 Session apply (interaction.rs)          ← layers 1–2
  • thought does not commit pending answer
  • answer after thought → clear draft (replace), ++replace_count
  • ToolUse → flush_all, reset replace_count
  • replace_count > 2 → cancel main path, keep draft, stop notice
        │
        ▼
 Transcript / stream UI
  one live Answer body (rewrites in place); channel switch still materializes
        │
        ▼
 Thinking body paint                     ← layer 4
  TextEdit base ink = text_secondary()
        │
 style Write gate (shared schema)        ← layer 3
  emit gate awaiting confirm → end turn; do not re-emit same gate
```

Harness event maps stay faithful. Grok only *exposes* thought↔answer loops; Claude and
others get the same draft/budget rules when channels interleave.

## Answer draft policy

`apply_answer_content_delta` / `apply_reasoning_content_delta` in
`crates/duckboard/src/area/interaction.rs`.

```
ReasoningDelta:  do NOT flush pending_text; append pending_reasoning
                 kind_switch = pending_text was open (UI structural; no commit)

ContentDelta:    flush pending_reasoning
                 if kind_switch && pending_text nonempty:
                   clear pending_text; answer_replace_count += 1
                 append delta to pending_text

ToolUse:         flush_all_pending; answer_replace_count = 0; then tool msg
TurnComplete:    flush_all_pending; answer_replace_count = 0
```

```rust
// ChatSession (in-memory only — not persisted)
pub answer_replace_count: u32; // reset each turn / tool / cancel settle
```

No session JSON schema change. Snapshot still folds current pendings only.

## Thrash budget (stream-ui)

**N = 2:** trip when `answer_replace_count > 2` (third answer-after-thought in a tool-free
span).

On trip (same turn as the replace that crosses the budget):

1. `handle.cancel()` — existing cancel path (`CancelPressed` / warm-runtime cancel)

2. Keep current `pending_text` as the answer (flush on settle like a normal cancel)

3. Append a short system (or equivalent non-answer) notice, e.g. that rewriting was
   stopped and the last draft was kept — not a second write-gate

4. Do not auto-start another agent turn

```rust
const ANSWER_REPLACE_BUDGET: u32 = 2;

// after increment on replace:
if session.answer_replace_count > ANSWER_REPLACE_BUDGET {
    // signal caller to cancel + notice; ignore further content/reasoning
    // deltas until streaming ends
}
```

Further `ContentDelta` / `ReasoningDelta` after trip are dropped (or no-op) until the turn
is no longer streaming, so a late-arriving thrash does not rebuild the draft after cancel
is requested.

Counter resets: tool use, turn complete, cancel settle, new send.

## Stream UI materialize

`kind_switch` remains structural even when thought does not flush answer (channel switch,
not “opposite buffer flushed”). Pure same-channel deltas stay tick-bounded. Doc/spec for
`chat/stream-ui` update that wording plus draft + thrash budget.

## Style emit-once (surgical)

One place: `crates/duckspec/content/schemas/style.md` under **Write gate** (loaded across
stages/providers). Add a short rule only — no Grok, no stage list:

> After emitting a write gate whose trailing `next` meta card awaits confirmation (`confirm` / `reject` / `revise`), **end the turn**. Do not re-emit or polish that gate in the same turn.

No per-template essays. Stage templates already point at style for write-gate shape.

## Thinking body fade

`TextEdit` optional base color (default `text_primary`); `view_thinking_block` sets
`base_color(theme::text_secondary())`. Header stays `text_muted()`. Answer body unchanged.

## Capability impact

```
| Path | Role |
| --- | --- |
| `chat/stream-ui` | Draft replace; channel-switch materialize; thrash budget + cancel + notice + drop post-trip deltas |
| `chat/transcript` | One answer span across thrash; Thinking body secondary ink |
| style schema | Emit-once write-gate rule (impl file under `content/schemas/`; not a cap) |
```

## Impact

- Code: `interaction` apply/cancel wiring; `agent_chat` thinking color; `text_edit` base
  color; style schema string

- Session JSON: no new fields; old multi-Text history untouched

- Cancel heat: reuses warm-runtime main cancel

- No harness protocol changes

## Decisions

- **N = 2** — one free rewrite, trip on third. Alternatives: 1 (aggressive), 3 (looser).

- **Cancel + keep draft + notice** — not ignore-deltas-only (would keep burning process
  heat).

- **Count replaces, not similarity** — locked non-goal.

- **Thrash in `chat/stream-ui`** — same apply path as replace; no new cap.

- **Emit-once in style only** — provider-neutral, all write-gate stages; not harness- or
  Grok-specific.

- **Draft policy in duckboard** — not `map_update` coalesce.

## Risks

- **True continuation after thought loses earlier text** → accepted; thrash is full
  restatements.

- **False trip on rare multi-rewrite legit answers** → N=2; tool boundary resets.

- **Emit-once is soft** → budget is the hard ceiling; style only reduces stimulus.

- **Tests assume flush-on-reasoning** → update with new contract.
