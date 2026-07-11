# Followup: drop cancel chip; freeform send while awaiting

User-led live pass after steps 06–07: chips work on Claude and Grok; cancel chip and
mid-question freeform send still wrong.

## Scope

Post-wire-fix live UI. Prior followup (`01`) hangs fixed. Design cancel/⌘⌫ table;
`caps/chat/fast-response`; `SendPressed` queue path when `is_streaming` (and thus while
awaiting); harness cancel encode still needed for esc/turn cancel.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | minor | quality | Drop cancel chip and ⌘⌫; esc already cancels | /ds-spec |
| 2 | major | soundness | Freeform send while question active queues / hard-cancels | /ds-spec |
```

## Issues

### 1. Drop cancel chip and ⌘⌫ - quality/minor

**Where:** design fast-response cancel; `crates/duckboard/src/fast_response.rs` cancel +
`resolve_cmd_backspace`; `caps/chat/fast-response` cancel/⌘⌫ scenarios; chip view in
`crates/duckboard/src/widget/agent_chat.rs`

**Why:** ⌘⌫ does not cancel in live use; esc esc already cancels the turn (and pending
choice via `handle.cancel` → `cancel_all`). Dedicated cancel chip is redundant noise.

**Action:** Remove cancel option, chip, and Cmd-Backspace binding from the shell. Keep
wire `UserChoiceAnswer::Cancelled` for turn cancel / freeform path. Specs drop cancel-chip
scenarios; `/ds-spec` then `/ds-step`.

### 2. Freeform send while question active queues / hard-cancels - soundness/major

**Where:** `crates/duckboard/src/area/interaction.rs` `SendPressed` — streaming +
non-empty input only queues; empty+queue hard-cancels. Live: agent reports
`permission stream closed` after force-send.

**Why:** While awaiting a choice the turn is still streaming, so typed text stages as
interrupt queue. Force-send cancels the process without first completing the parked choice
in harness-normal form (Claude control deny / Grok `skip_interview`), so the question tool
looks failed instead of skipped.

**Action:** When `is_awaiting_user` and the user submits freeform text: complete the
pending choice as cancelled (harness-normal), clear chips, then send that text as a normal
chat turn immediately (not queue + double-enter). Spec the behavior; implement via
`/ds-spec` → `/ds-step`.

## Outcome

Agreed chips work; drop cancel chrome; freeform-while-awaiting must cancel the tool
cleanly and send at once. Plan and code unchanged in this write. Not archive-ready until
issue 2 lands (issue 1 is product cleanup in the same pass). Suggested next: `/ds-spec`
for both issues, then `/ds-step`.
