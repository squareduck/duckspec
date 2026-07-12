# Persist context usage - Design

Persist last-known context token totals on the chat session JSON so restart rehydrates the
composer meter from disk, without changing harness reporting or footer formatting.

## Approach

Usage stays harness-driven and ephemeral until observed. Once observed, the **total** used
for the meter is written onto `ChatSession` and rides every existing save. Load seeds the
live counters. No separate store, no window persistence, no footer chrome change.

```
UsageUpdate(input, output)
        │
        ▼
  agent_input/output  (live meter)
        │  write-through sum
        ▼
  ChatSession.context_tokens  ──save──►  *.json
        ▲
        │  from_session
  app restart / load
```

Denominator remains `model_context_window(effective_model)` from catalog +
`selected_model` (already persisted).

## Session field

Add one numerator on the durable session model in `crates/duckboard/src/chat_store.rs`:

```rust
// ChatSession + PersistedSession
/// Last known context fill numerator (input+output). Default 0 for
/// legacy files and sessions that never saw UsageUpdate.
#[serde(default)]
pub context_tokens: usize,
```

Wire through `ChatSession::new` (0), `load_sessions_for`, and `save_session` like
`selected_model`.

Round-trip tests belong next to existing persistence tests in `chat_store.rs`.

## Live rehydrate and write-through

`AgentSession` keeps `agent_input_tokens` / `agent_output_tokens` for the meter.

```
| Moment | Behavior |
| --- | --- |
| `from_session` | `agent_input_tokens = session.context_tokens`; `agent_output_tokens = 0` |
| `UsageUpdate` | same `if > 0` merge as today on live counters; then `session.context_tokens = input + output` |
| Footer | unchanged: `context_tokens: input + output`, `context_max` from catalog |
```

Sketch (`main.rs` UsageUpdate arm):

```rust
AgentEvent::UsageUpdate { input_tokens, output_tokens } => {
    if input_tokens > 0 {
        ax.agent_input_tokens = input_tokens;
    }
    if output_tokens > 0 {
        ax.agent_output_tokens = output_tokens;
    }
    ax.session.context_tokens =
        ax.agent_input_tokens + ax.agent_output_tokens;
}
```

No dedicated save on usage. Turn-boundary `save_session`, eager flush (when
`needs_flush`), and other existing saves already serialize `ChatSession` — once the field
is on the session, they include it. Mid-turn crash may keep the **previous** turn’s total
(same “correct-ish” budget as a lost message tail).

Do **not** set `needs_flush` solely for usage: avoid rewriting the full session JSON on
every token telemetry tick when no messages changed.

## Capability surface

Extend **`chat/persistence`**: durable session includes last-known context usage; load
restores it; missing field → 0.

No change to `composer-footer` progressive readout or `harness/model-picker` window rules.
Footer already displays whatever numerator it is given.

## Impact

```
| Area | Effect |
| --- | --- |
| Session JSON | Additive `context_tokens` (default 0); old files load unchanged |
| `chat_store` | Field + load/save mapping + round-trip test |
| `AgentSession::from_session` | Seed live counters from session |
| `UsageUpdate` handler | Write-through sum onto session |
| Caps | `chat/persistence` spec/doc only |
| duckchat / harnesses | None |
```

## Decisions

- **Single total, not input/output split** — meter only needs the sum; split would mirror
  event shape without product value. Alternatives: two fields (rejected as noise); store
  on `AgentSession` only (rejected — not on disk).

- **Write-through on UsageUpdate, persist via existing saves** — matches
  `agent_session_id` / model (mutate session, save on turn boundary). Alternative:
  snapshot only inside `save_session` from a side channel (rejected — save only sees
  `ChatSession`).

- **Do not flush on usage alone** — avoids chatty disk writes; last completed turn’s total
  is enough post-restart.

- **No window on disk** — non-goal; catalog + `selected_model` already recover denominator
  when the model is known.

## Risks

- **Kill mid-turn before TurnComplete** → meter may show prior turn’s total after restart
  → accepted; same durability class as eager message flush bounds.

- **Model switch without new usage** → old numerator vs new window can mis-scale →
  accepted non-goal; same as live today until the next report.

- **Usage arrives only after TurnComplete** (if a harness ever does that) → total might
  lag one save → rare; next turn’s save still lands it; if observed, fold usage into the
  same path that already saves session id updates.
