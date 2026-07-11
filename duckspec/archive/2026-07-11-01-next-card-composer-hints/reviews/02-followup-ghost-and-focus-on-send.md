# Followup: ghost and focus on send

User-led pass: next-action ghost lingers after send while streaming, and empty Enter
sometimes drops chat input focus.

## Scope

Post–step-06 followup on `next-card-composer-hints`: empty-composer ghost vs streaming;
focus retention on empty Enter / next-action send (`next_ghost_text`, tab-marker layout,
`SendPressed` / `send_prompt_text`).

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | quality | Ghost stays after send / mid-turn | /ds-step |
| 2 | major | quality | Empty Enter loses input focus | /ds-step |
```

## Issues

### 1. Ghost stays after send / mid-turn - quality/major

**Where:** `crates/duckboard/src/default_prompts.rs` (`next_ghost_text`);
`crates/duckboard/src/widget/agent_chat.rs` (placeholder); `next_actions` refresh on turn
complete vs turn start

**Why:** After the user sends (empty Enter next action or typed message), the composer is
empty and the main turn streams, but the previous next-action send text still appears as
ghost. Empty Enter and Tab are already disarmed while streaming; the ghost alone still
looks armed and confuses mid-turn.

**Action:** Hide or clear next-action ghost immediately when a turn is in progress (gate
on streaming and/or clear `next_actions` on send), consistent with tab marker and empty
submit. `/ds-step` / `/ds-apply`.

### 2. Empty Enter loses input focus - quality/major

**Where:** `crates/duckboard/src/widget/agent_chat.rs` (conditional `row![input, ⇥]` when
tab marker visible); `SendPressed` / `send_prompt_text` (no post-send `focus_chat_input`,
unlike Tab cycle)

**Why:** With multi next, empty Enter starts streaming and hides the tab marker, so the
input leaves a `Row` wrapper and becomes a bare `TextEdit`. That tree-shape change drops
iced focus. It feels intermittent because it depends on multi-next chrome being visible.

**Action:** Keep a stable input-row structure across marker show/hide, and/or refocus the
chat input after empty-next send. `/ds-step` / `/ds-apply`.

## Outcome

Agreed on two mid-turn composer bugs. Suggested next: `/ds-step` (one step can cover
both). Not archive-ready until they land.
