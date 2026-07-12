# Followup: band contrast tune

User-led check after steps 04–05: landmark scroll/streaming OK; last-Answer band contrast
is too strong.

## Scope

Live visual pass on last-Answer band after `bg_chat_last_answer` strengthen (step 05).

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | minor | quality | Last-Answer band too strong | /ds-step |
```

## Issues

### 1. Last-Answer band too strong - quality/minor

**Where:** `crates/duckboard/src/theme.rs` — `bg_chat_last_answer`

**Why:** After step 05 (~55% surface + 45% elevated), the full-width band is too
prominent; landmarks otherwise good.

**Action:** Dial back toward a mid setting (e.g. full `bg_surface`, or a milder
surface/elevated mix) — still clearly above `bg_chat_area`, not card chrome.

## Outcome

One visual tune remaining before archive. No new specs; short rework step.
