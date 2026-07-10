# Audit scope in critique templates

User-led pass after rework: review and followup content refer to bare `ds audit`
(project-wide) where change-scoped `ds audit <change>` or no audit at all is correct.

## Scope

Post-implementation followup on change `review-followup-workflow`: shipped critique
templates and schemas vs apply/archive audit conventions. Content only under
`crates/duckspec/content/templates/` and `schemas/` for review and followup.

## Summary

```
| # | sev | lens | title | → next |
|---|-----|------|-------|--------|
| 1 | major | fidelity | Critique templates/schemas say bare `ds audit` | /ds-step |
```

## Issues

### 1. Critique templates/schemas say bare `ds audit` — fidelity/major

**Where:** `crates/duckspec/content/templates/review.md` Role and “don’t re-verify”
guidance; `schemas/review.md` and `schemas/followup.md` well-formedness framing. Contrast:
`templates/apply.md` correctly uses `ds audit <change>`; `templates/archive.md` correctly
uses bare `ds audit` for whole-project health.

**Why:** Bare `ds audit` is project-wide integrity, not change progress. Agents reading
review/followup can run full-project audit mid-critique and treat unrelated archive or
other-change noise as “this change isn’t well-formed.” Followup’s job is only the followup
document (`ds format` / `ds check` on that file); it should not imply running audit at
all.

**Action:** Tighten review/followup templates and schemas: if naming a command for change
well-formedness, use `ds audit <change>`; keep followup free of audit run instructions
(conceptual “static tooling” is fine only if it doesn’t imply bare `ds audit`). No
product-code change expected — content rework via `/ds-step`.

## Outcome

Agreed: bare `ds audit` in critique content is wrong for change-scoped intent; followup
should not run audit. Plan and product code were not changed in this session. Suggested
next: `/ds-step` to retarget wording.
