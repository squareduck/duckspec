# Followup: Grok freeform-while-awaiting hangs

User-led live pass after steps 08–09: Claude freeform-while-awaiting works; Grok hangs and
does not answer the freeform text after esc.

## Scope

Post-apply live UI on Grok 4.5 vs Claude. Freeform path in
`crates/duckboard/src/area/interaction.rs` (`plan_freeform_while_awaiting` +
`SendPressed`); ACP cancel write then `handle.cancel()`; queue auto-flush on
`TurnComplete`. Prior followups 01–02 wire/cancel-chip fixed.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | soundness | Grok freeform-while-awaiting hangs; freeform not answered | /ds-step |
```

## Issues

### 1. Grok freeform-while-awaiting hangs - soundness/major

**Where:** `crates/duckboard/src/area/interaction.rs` freeform path (answer `Cancelled` →
queue freeform → `handle.cancel()`); `crates/duckchat/src/acp/turn.rs` write
skip_interview then kill if cancel token set; Grok warm re-spawn after interrupt

**Why:** Claude with the same path cancels the question and continues. Grok stays on the
question tool until esc esc; freeform text may appear as a user message but gets no agent
reply after cancel. Likely race/kill: skip_interview then immediate process kill / re-warm
leaves Grok unable to finish the freeform turn.

**Action:** Fix freeform-while-awaiting for Grok without regressing Claude — e.g. complete
choice as cancelled (harness-normal) and arm freeform for flush on natural turn end,
deferring hard interrupt; or ensure skip_interview is fully accepted before kill and that
the next turn re-warms cleanly. Spec if behavior splits by harness; implement via
`/ds-step` / `/ds-apply`.

## Outcome

Agreed Claude freeform is good; Grok freeform hang is a remaining blocker. Plan and code
unchanged in this write. Not archive-ready until Grok freeform cancels the question and
the freeform message is answered without a stuck esc-esc loop. Suggested next: `/ds-step`
(and `/ds-spec` only if the contract must split by harness).
