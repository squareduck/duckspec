# Review schema

A review is a **judgment** on a change — the read that static verification can't
do. `ds audit <change>` and `ds check` already prove a change is well-*formed*:
schemas valid, scenarios covered, backlinks resolved. (Bare `ds audit` is
whole-project health, not change progress.) A review asks the harder question:
is the change well-*conceived* and well-*made*? It records that judgment as a
document a reader or agent can act on. Reviews live under
`changes/<name>/reviews/NN-review-<slug>.md` and form an append-only,
chronological log shared with followups.

**Producing this document is the whole job of `/ds-review`.** Applying fixes
(plan, code, or templates) is a later choice by the user — `/ds-spec`,
`/ds-step`, ignore, or an explicit in-place request — not part of writing the
review.

## What a review examines

A change is a chain — each layer is built on the one above it:

```
proposal ──→ design ──→ caps (spec/doc) ──→ code
```

A review reads *down* this chain to the deepest artifact that exists yet, so it
applies at any stage: a proposal-only change is reviewable (is the plan sound?),
and so is a fully-implemented one (does the code realize it, cleanly?). The
review never re-checks what static verification owns — it judges the thinking and
the craft.

It judges along three **lenses**:

- **soundness** — is this artifact, on its own terms, *right*? The proposal solves
  a real problem the right way; the design's architecture holds up; a cap models
  true and complete behavior; the code is correct.
- **fidelity** — does each layer faithfully realize the one above it? design↔
  proposal, caps↔design, code↔(design + caps). A divergence that *improves* on
  the upstream intent is worth noting but is not a defect; one that erodes it is.
- **quality** — is it well-*made*? Simple as the problem allows, idiomatic for
  this codebase and language, free of code smells. Mostly a code lens, but a
  tangled design or a bloated spec qualifies too.

## Structure

Write for two read modes: a **Summary** table for triage, then structured detail
under **Findings**. Chat presentation should match the Summary table.

```markdown
# <Review Title>

<1-2 sentence summary: what was reviewed, at what stage, and the headline verdict>

## Scope

<what this review covers — the artifacts and/or code examined, and the stage the
change is at (proposal-only, mid-implementation, post-implementation). Name the
deepest layer reached down the chain.>

## Summary

| # | sev | lens | title | → next |
|---|-----|------|-------|--------|
| 1 | critical | soundness | State comparison inverted on callback | /ds-spec |
| 2 | major | quality | Token exchange reimplements retry helper | /ds-step |

## Findings

### 1. <Finding title> — <lens>/<severity>

**Where:** <`path:line` for code, or artifact + section for an upstream layer>

**Why:** <why it matters long-term if frozen as-is>

**Action:** <concrete recommended action or next stage — not work already performed
in this session>

## Open questions

<genuine unknowns you could not resolve yourself — a product-intent call, a
decision only the human can make. Omit this section if there are none.>

## Verdict

<an aggregate judgment scoped to the stage reviewed — not a tally of findings. Is
the thinking sound, and is the realization (as far as it exists) faithful and
well-made? Say plainly what should improve before this is accepted as done.>
```

Number findings in the Summary table and use the same numbers in Findings headings
so the two surfaces stay aligned. The `→ next` column recommends the stage that
would own the work (`/ds-spec` for new or changed behavior, `/ds-step` for
restructure, `/ds-archive` when ready, or `ignore`).

## Severity

Severity measures how much a finding hurts the codebase long-term if it is frozen
as-is — the drag it leaves, not whether it breaks today. It is **independent of
lens**: a quality finding earns `critical` on its own merits, exactly as a
soundness one does. A cleanup is **not** automatically minor — duplication or
accidental complexity in load-bearing code is `major` or worse, because it
compounds every time that code is touched.

- **critical** — leaves lasting structural harm: a wrong design decision, code
  that contradicts its spec, a core abstraction that doesn't hold, or a
  duplication / complexity / coupling failure that will force scattered edits and
  compound for the life of the code. Address before this is accepted as done.
- **major** — a real problem that leaves durable drag if frozen: duplicated logic,
  needless indirection, a function doing several jobs, logic in the wrong layer —
  something a future maintainer pays for repeatedly, even though the change still
  works.
- **minor** — genuinely low-cost: a localized nit, a naming choice, a small polish
  whose absence a maintainer would never feel. Reserve this for findings that
  truly don't compound — not for real cleanup you'd rather not rank higher.

Rate by lasting harm, not by lens. Simplicity and long-term maintainability are
primary acceptance criteria, so a finding that erodes them is never discounted for
being "just" craft. Still reserve `critical` for genuine structural harm — inflated
severity trains readers to ignore you, and grading every nit as `major` does the
same.

## Rules

- H1 title is required.
- A summary paragraph directly follows the H1.
- The body is freeform markdown — the sections above are recommended, not enforced
  by `ds check`. A review validates against the document schema only.
- New creates use a `review-` slug prefix (`NN-review-<slug>.md`); legacy files
  without a kind prefix remain valid.

## Quality

- **Document first.** The review file is the deliverable; applying fixes is out of
  band until the user chooses a next step.
- **Scannable first.** A reader should triage from Summary without reading every
  finding body. Keep titles short; put depth under **Where** / **Why** / **Action**.
- **Recommend, don't apply.** Action describes what should happen next; it does not
  narrate edits performed during `/ds-review`.
- **Judge, don't re-verify.** Don't spend findings on what
  `ds audit <change>` / `ds check` already prove (unresolved backlinks,
  uncovered scenarios, invalid schemas). Spend them on soundness, fidelity, and
  craft — the things only judgment catches.
- **The verdict is an aggregate, not a maximum.** Form a holistic read. Five minor
  quality findings in a small, load-bearing component is a *different* signal than
  one stray nit — the verdict must reflect the gestalt, not just the worst single
  finding. A change can be free of any `critical` finding and still not be ready to
  accept.
- **Resolve before you file.** If you have the tools to answer your own question —
  grep the callers, read the test, check the type — do it before filing. File a
  finding only for what survives that check; raise an Open question only for what
  genuinely needs a human. Don't hand back homework you could have done.
- **A finding is actionable; an observation is not.** Name the artifact or
  `path:line`, state why it matters, recommend a concrete action. Praise, notes,
  and "diverges from the design — and the divergence is correct" belong in prose
  or are dropped. They are not findings, and padding the list with them buries the
  signal.
- **Don't flag** what a reviewer shouldn't: pre-existing issues outside the
  change, intentional and correct divergence, anything a linter / type-checker /
  compiler catches, pedantic nits a senior engineer would wave through, or matters
  of pure taste with no codebase convention behind them.
- **Reason, then rate.** Write the critique first; assign the lens and severity
  after. Rating first invites you to rationalize a number.
- **Firm, not ceremonial.** Reviews are advisory — nothing in the system blocks on
  one — but advisory to the *human*, not soft in *judgment*. You are the last
  quality bar before this work is accepted as done; hold it like one. A review
  that only says "looks good" wasn't worth writing.

## Formatting

After writing or updating this artifact, run `ds format <path>` to apply canonical
formatting (line wrap, indentation, blank lines).

Use fenced code blocks for tables and diagrams; add a `<language>` tag to fences
that contain real code.

## Example

```markdown
# Post-implementation review: Google OAuth login

Reviewed the `auth/google` implementation end-to-end against its design and spec.
Sound and faithful, but the callback module carries enough avoidable complexity
that I wouldn't freeze it as-is.

## Scope

The `caps/auth/google` spec and design, the change's steps, and the code under
`src/auth/google/`. Post-implementation: the full chain down to code.

## Summary

| # | sev | lens | title | → next |
|---|-----|------|-------|--------|
| 1 | critical | soundness | State comparison inverted on callback | /ds-spec |
| 2 | major | quality | Token exchange reimplements retry helper | /ds-step |
| 3 | minor | fidelity | Callback splits auth logic across modules | /ds-step |

## Findings

### 1. State comparison inverted on callback — soundness/critical

**Where:** `src/auth/google/callback.rs:42`

**Why:** Every callback passes the forged-state check; contradicts the "Callback
rejects a forged state" scenario, whose test only asserts the param is present.

**Action:** Compare against the stored authorize-time value; tighten the test to
assert a mismatch is rejected.

### 2. Token exchange reimplements retry helper — quality/major

**Where:** `callback.rs:70-110`

**Why:** Hand-rolled retry-with-backoff duplicates `http::retry`, drops jitter,
and is a second place to fix bugs.

**Action:** Call `http::retry` instead.

### 3. Callback splits auth logic across modules — fidelity/minor

**Where:** error mapping in `mod.rs` vs design boundary on `callback`

**Why:** Erodes the single-module boundary the design chose.

**Action:** Fold error mapping back into `callback`.

## Verdict

Mid-grade: the flow is sound and matches the design, but the inverted state check
is a foundational bug and the duplicated retry loop is the kind of debt that
compounds. Resolve both before accepting; the boundary nit is optional.
```
