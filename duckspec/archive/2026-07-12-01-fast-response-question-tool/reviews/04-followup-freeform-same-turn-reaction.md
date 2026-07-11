# Followup: freeform still same-turn after skip

User-led pass after step 10 (no hard interrupt): Claude freeform cancel verbosity is
acceptable; Grok is worse — continues the open turn after skip and does not treat freeform
as a clean next user turn.

## Scope

Live freeform-while-awaiting on Grok vs Claude post-`interrupt_turn: false`. Wire still
`skip_interview` / deny; freeform only queued for `TurnComplete`. Allowed Grok outcomes
include `chat_about_this` (not implemented). Prior followups 01–03 wire, cancel-chip, and
hang notes.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | soundness | Grok freeform: skip continues same turn; freeform not a clean next send | /ds-step |
| 2 | minor | quality | Claude verbose on cancel before freeform | ignore |
```

## Issues

### 1. Grok freeform: skip continues same turn; freeform not a clean next send - soundness/major

**Where:** freeform path in `crates/duckboard/src/area/interaction.rs` (answer `Cancelled`
→ queue → no interrupt); Grok `skip_interview` encode in
`crates/duckchat/src/acp/ask_user.rs`; live: tool stays in activity, then Thinking about
“enough answers” / “something else” without a proper freeform user turn first

**Why:** Without hard kill, Grok accepts skip and keeps generating on the open turn.
Freeform waits for natural turn end, so the agent reacts to cancel (and confuses freeform)
instead of receiving freeform as the next user message promptly. User reports this as
worse than the prior hang.

**Action:** Deliver freeform as a real next user turn after a clean choice cancel — e.g.
after skip/deny is written, end the turn reliably without the kill-race from followup 03,
**or** probe/use Grok `chat_about_this` (or equivalent) so freeform is the choice result.
Spec if wire gains a freeform answer shape; `/ds-spec` only if needed, then `/ds-step`.

### 2. Claude verbose on cancel before freeform - quality/minor

**Where:** same freeform path on Claude (deny, turn continues briefly)

**Why:** Agent narrates the cancelled question before freeform flushes on turn end.

**Action:** None — user accepts. Record only.

## Outcome

Claude freeform acceptable; Grok freeform still a blocker (worse after deferring
interrupt). Plan and code unchanged in this write. Not archive-ready until Grok freeform
cancels the tool and the freeform text is answered as a normal follow-up without same-turn
cancel monologue eating the UX. Suggested next: `/ds-step` (probe wire if using
`chat_about_this`).
