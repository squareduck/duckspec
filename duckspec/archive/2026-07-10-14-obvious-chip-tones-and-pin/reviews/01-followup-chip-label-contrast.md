# Chip label contrast

User-led pass after implementing obvious chip tones and bottom pin: chip label ink is too
faint in both light and dark themes.

## Scope

Post-implementation followup on change `obvious-chip-tones-and-pin`: shipped chrome view
and theme tints, with live UI checked on multi-option lifecycle plus Confirm/Reject gate.
Deepest layer: product code (`agent_chat` chip labels).

## Summary

```
| # | sev | lens | title | → next |
|---|-----|------|-------|--------|
| 1 | major | quality | Obvious-chip label text too faint (both themes) | /ds-step |
```

## Issues

### 1. Obvious-chip label text too faint (both themes) — quality/major

**Where:** `crates/duckboard/src/widget/agent_chat.rs` — `view_obvious_chip` (label color
from `theme::text_muted()` with alpha × 0.95)

**Why:** Numbered, enter, and reject chips all share one muted label path. On the already
soft ~8% tinted chip fills, overlay/muted ink is hard to read in dark and light themes,
which undercuts the scannability the change was meant to improve.

**Action:** Raise label contrast for all obvious-chrome chips (e.g. use `text_secondary`
or `text_primary` at full alpha, still one path for every tone). Keep quiet fill tints.
Plan and land via `/ds-step` / `/ds-apply` — no new capability required unless contrast is
later elevated into the cap prose.

## Outcome

Agreed chip labels need stronger contrast in both themes; fill role tints stay quiet. Plan
and code were not changed in this session. Suggested next: `/ds-step` to plan the
label-color fix, then `/ds-apply`. Not archive-ready until the contrast fix lands.
