# Review schema

A review is a **judgment** on a change: whether it is well-conceived and
well-made. Static checks (`ds check`, `ds audit <change>`) own well-formed;
the review owns thinking and craft. Append-only under
`duckspec/changes/<name>/reviews/NN-review-<slug>.md` (shared log with followups).

## Structure

```markdown
# <Review Title>

<1-2 sentence summary: what was reviewed, stage, headline verdict>

## Scope

<artifacts and/or code examined; stage; deepest layer on the chain>

## Summary

| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | critical | soundness | <short title> | /ds-spec |
| 2 | major | quality | <short title> | /ds-step |

## Findings

### 1. <Finding title> - <lens>/<severity>

**Where:** <`path:line` or artifact + section>

**Why:** <lasting cost if frozen as-is>

**Action:** <recommended next stage or approach - not work already done>

## Open questions

<genuine unknowns only a human can settle. Omit if none.>

## Verdict

<aggregate judgment for this stage - gestalt, not max severity alone>
```

```
proposal --> design --> caps (spec/doc) --> code
```

Read down the chain as far as artifacts exist. Number Summary rows and Findings
headings the same. Empty Summary (header row only, no data rows) is valid when
there are no lasting findings - Verdict still states readiness. Column
`→ next` is a real stage or path (`/ds-spec`, `/ds-step`, `/ds-archive`, or a
short approach that implies one) - not "already fixed" and not `ignore`.

## Lenses

- **soundness** - right on its own terms (problem, architecture, model, code)
- **fidelity** - each layer realizes the one above; improving divergence is not
  a defect, eroding divergence is
- **quality** - simple, idiomatic, maintainable (mostly code; tangled design or
  bloated specs qualify)

## Severity

Lasting harm if frozen as-is - independent of lens. Rate by drag, not "does it
run today."

| Level | Meaning |
| --- | --- |
| critical | Lasting structural harm; address before accepting as done |
| major | Real durable drag if frozen (duplication, wrong layer, multi-job units) |
| minor | Low-cost polish that still compounds if ignored - not "file and forget" |

Do not inflate severity (teaches readers to ignore you). Do not discount quality
findings as "just craft" when they erode maintainability.

## Rules

- Path: `duckspec/changes/<name>/reviews/NN-review-<slug>.md` (`review-` prefix
  on create; legacy unprefixed files remain valid)
- H1 title required; non-empty summary paragraph follows it
- Body freeform; Structure is recommended (document schema only for `ds check`)
- Append-only log - new file per review; do not renumber or rewrite history

## Quality

Cold-reader shape of a finished file - not which findings to invent (that is
the `/ds-review` stage template).

- **Scannable first.** Triage from Summary; depth under Where / Why / Action
- **Actionable rows only.** Each Summary row has location, lasting cost, and a
  concrete Action with a real `→ next`. Observations without a path, praise, and
  improving divergences are prose (Verdict), not rows
- **No noop next.** `→ next` is never `ignore` or equivalent
- **Empty Summary is valid.** Clean freeze-ready Verdict with Scope showing what
  was examined is a complete review
- **Verdict is aggregate.** Gestalt readiness, not the single worst row
- Body markdown follows `style` (load only if not already in context)

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

```markdown
# Post-implementation review: Google OAuth login

Reviewed `auth/google` end-to-end against design and spec. Flow is sound; the
callback carries avoidable complexity and one foundational bug.

## Scope

`caps/auth/google` spec and design, change steps, and `src/auth/google/`.
Post-implementation: full chain to code.

## Summary

| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | critical | soundness | State comparison inverted on callback | /ds-spec |
| 2 | major | quality | Token exchange reimplements retry helper | /ds-step |

## Findings

### 1. State comparison inverted on callback - soundness/critical

**Where:** `src/auth/google/callback.rs:42`

**Why:** Forged-state check always passes; contradicts the rejection scenario.

**Action:** Compare to authorize-time value; tighten the test via `/ds-spec`
then `/ds-step` if the rejection scenario is missing.

### 2. Token exchange reimplements retry helper - quality/major

**Where:** `callback.rs:70-110`

**Why:** Duplicates `http::retry` without jitter; second place to fix bugs.

**Action:** Call `http::retry`.

## Verdict

Not ready to freeze: fix the inverted state check and drop the hand-rolled
retry.
```
