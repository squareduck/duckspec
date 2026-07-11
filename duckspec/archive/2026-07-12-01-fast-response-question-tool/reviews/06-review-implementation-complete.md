# Review: implementation complete (custom answer + chips)

Post-apply review after 15 steps and a live freeform pass. Audit clean; custom answer and
awaiting chrome behave as intended. Residual: design/proposal still describe the cancelled
chip model; chip pick does not clear typed composer text.

## Scope

Proposal, design (stale cancel/⌘⌫), change caps (current truth), steps 01–15, duckchat ACP
choice encode, Claude/Grok edges, duckboard freeform + visibility + tint. Followups 01–05.
Deepest layer: source. `ds audit` clean; clippy clean on touched crates.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | minor | fidelity | design/proposal still describe cancel chip and pre-custom freeform | ignore |
| 2 | minor | quality | Chip option activation leaves typed composer text | ignore |
```

## Findings

### 1. design/proposal still describe cancel chip and pre-custom freeform - fidelity/minor

**Where:** `proposal.md` intent (⌘⌫ cancel); `design.md` cancel shell field, externally
tagged Accepted/SkipInterview, freeform as later non-goal

**Why:** Caps and code are the product truth: no cancel chip; freeform completes as custom
answer on the question tool; wire is flat `outcome` for Grok. Stale design confuses
archive readers who open design first. Followups already document the evolution; this is
doc-history drift, not eroding caps→code fidelity.

**Action:** Optional brief design note before archive, or leave historical. Not a code
fix. `ignore` unless a doc-only polish is desired.

### 2. Chip option activation leaves typed composer text - quality/minor

**Where:** `crates/duckboard/src/area/interaction.rs` `activate_fast_response` (clears
shell only); freeform submit path clears the composer

**Why:** While awaiting, chips stay visible during custom-answer typing. Activating ⌘n
after partial freeform answers the chip but leaves text in the composer, which could be
sent as a later turn by accident.

**Action:** Optional: clear composer (or discard partial freeform) on option activation.
Low cost polish. `ignore` or a tiny follow-up step if UX matters.

## Verdict

**Accept / archive-ready.** Intent (mid-turn structured choice for Claude and Grok via
shared ACP, custom freeform as answer, awaiting chrome) is realized and live-proven.
Residual findings are doc drift and optional UX polish, not structural risk. Suggested
next: `/ds-archive` unless finding 2 is fixed first.
