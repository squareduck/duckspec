# Settings group heading hierarchy

Post-implementation UX followup: Settings group headings (`Global` / `This project`)
render at the same size as field titles. Fix with `font_lg` for groups and extra space
between Global and This project.

## Scope

Settings layout from step 05 (`crates/duckboard/src/area/settings.rs`, `theme.rs` type
scale). No cascade, catalog, or send-gate behavior.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | medium | ux | Settings group headings same size as fields | /ds-step |
```

## Issues

### 1. Settings group headings same size as fields - ux/medium

**Where:** `crates/duckboard/src/area/settings.rs` — `Global` and `This project` use
`theme::font_md()`, matching field titles (`UI Font`, `Default Model`, …). Theme documents
a third tier (config + 2) but only exposes `font_sm` / `font_md`
(`crates/duckboard/src/theme.rs`).

**Why:** Parent groups do not read above their children; the intended Global vs This
project hierarchy is lost in the UI.

**Action:** Add `theme::font_lg()` as `ui_size() + 2`. Use it for group headings only;
keep field titles at `font_md`. Increase vertical space between the end of the Global
block and the This project heading (more than a single `SPACING_XL`, or an extra gap after
Global) so the two scopes scan as peer sections.

## Outcome

One presentation fix agreed; specs unchanged. Plan a small implementation step, then
apply. Not archive-ready until the hierarchy fix lands.
