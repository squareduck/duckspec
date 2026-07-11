# Re-review after Interaction drain fix

Re-checked priming Setup collapse after step 03. Prior critical finding (expand timer
never scheduled on the chat UI path) is fixed and regression-covered. Change is
archive-ready.

## Scope

Full chain: `proposal.md`, `design.md`, `caps/chat/transcript` deltas, steps 01–03, review
01, and code under `crates/duckboard` (`agent_chat`, `text_edit/state`,
`area/interaction`, `main`). Post-fix re-review; `ds audit` clean.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
```

No open findings.

## Findings

(none)

## Verdict

**Accept / archive-ready.**

Prior critical (review 01): expand staged `pending_priming_recollapse` but
`Message::Interaction` early-returned `route_interaction` without draining. **Resolved**
in step 03: `route_interaction` batches `take_pending_priming_recollapse`; fall-through
drain remains a safe no-op; `InteractionState::take_pending_priming_recollapses` plus
`expanding_priming_setup_drains_into_recollapse_jobs` lock the expand → job contract.

Collapse defaults, Setup presentation, generation-gated re-collapse, and `@spec` coverage
stay coherent. No new soundness, fidelity, or quality issues that survive self-resolution.
