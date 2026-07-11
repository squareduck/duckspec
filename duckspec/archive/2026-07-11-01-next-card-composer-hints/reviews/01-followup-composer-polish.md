# Followup: composer polish

User-led pass after apply: calm meta-card tint for both themes, and fix tab-available
marker placement.

## Scope

Post-implementation followup on `next-card-composer-hints`: theme `META_CARD_BG`, Answer
`LineBgKind::MetaCard`, and empty-composer tab marker in `agent_chat` view.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | minor | quality | Meta-card tint too loud | /ds-step |
| 2 | minor | quality | Tab-available marker misplaced | /ds-step |
```

## Issues

### 1. Meta-card tint too loud - quality/minor

**Where:** `crates/duckboard/src/theme.rs` (`META_CARD_BG` / `meta_card_bg`); Answer line
backgrounds via `LineBgKind::MetaCard`

**Why:** Tint should be a quiet band for scanning gates/handoffs. Current dark and light
swatches read as a strong quote wash rather than calm differentiation from ordinary Answer
text, search Match, and diff hunks.

**Action:** Choose calmer colors for both themes (lower chroma, closer to surface/mantle)
and verify in dark and light; implement via `/ds-step` / `/ds-apply`.

### 2. Tab-available marker misplaced - quality/minor

**Where:** `crates/duckboard/src/widget/agent_chat.rs` — `view_next_tab_marker` and the
composer column layout that places it under the input

**Why:** When multiple next actions are armed, the `⇥` affordance should sit with the
empty-composer next chrome (ghost), not as a separate full-width strip that feels detached
from the active send text.

**Action:** Reposition the marker to align with the next-action affordance; keep
visibility rules (`len > 1`, empty input, idle). `/ds-step` / `/ds-apply`.

## Outcome

Agreed on two polish fixes from tryout. Specs unchanged; rework is layout and theme only.
Suggested next: `/ds-step`. Not archive-ready until these land (or are explicitly
ignored).
