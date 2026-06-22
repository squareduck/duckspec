# Review schema

A review is a **judgment** on a change — the read that static verification can't
do. `ds audit` and `ds check` already prove a change is well-*formed*: schemas
valid, scenarios covered, backlinks resolved. A review asks the harder question:
is the change well-*conceived* and well-*made*? It records that judgment as a
document a reader or agent can act on. Reviews live under
`changes/<name>/reviews/NN-<slug>.md` and form an append-only, chronological log.

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

```markdown
# <Review Title>

<1-2 sentence summary: what was reviewed, at what stage, and the headline verdict>

## Scope

<what this review covers — the artifacts and/or code examined, and the stage the
change is at (proposal-only, mid-implementation, post-implementation). Name the
deepest layer reached down the chain.>

## Findings

### <Finding title> — <lens>/<severity>

<what the issue is, where it lives (`path:line` for code, the artifact + section
for an upstream layer), why it matters long-term, and the recommended action>

## Open questions

<genuine unknowns you could not resolve yourself — a product-intent call, a
decision only the human can make. Omit this section if there are none.>

## Verdict

<an aggregate judgment scoped to the stage reviewed — not a tally of findings. Is
the thinking sound, and is the realization (as far as it exists) faithful and
well-made? Say plainly what should improve before this is accepted as done.>
```

## Severity

Severity measures how much a finding hurts the codebase long-term if it is frozen
as-is. It is **independent of lens** — a quality finding can be `critical` (a core
abstraction that's badly wrong) and a fidelity finding can be `minor`.

- **critical** — undermines the change at its foundation: a wrong design decision,
  code that contradicts its spec, a core abstraction that doesn't hold. Address
  before this is accepted as done.
- **major** — a real problem that leaves lasting drag if frozen, but the change is
  not fundamentally broken.
- **minor** — a small improvement, cleanup, or nit.

Reserve `critical` for what genuinely breaks the change's foundation. Inflated
severity trains readers to ignore you.

## Rules

- H1 title is required.
- A summary paragraph directly follows the H1.
- The body is freeform markdown — the sections above are recommended, not enforced
  by `ds check`. A review validates against the document schema only.

## Quality

- **Judge, don't re-verify.** Don't spend findings on what `ds audit`/`ds check`
  already prove (unresolved backlinks, uncovered scenarios, invalid schemas).
  Spend them on soundness, fidelity, and craft — the things only judgment catches.
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

## Findings

### State comparison is inverted on the callback — soundness/critical

`src/auth/google/callback.rs:42` compares the returned `state` to a freshly
generated value, not the one stashed at authorize time, so every callback
passes the check. This contradicts the "Callback rejects a forged state"
scenario, whose test only asserts the param is present. Compare against the
stored value and tighten the test to assert a mismatch is rejected.

### Token exchange reimplements the shared retry helper — quality/major

`callback.rs:70-110` hand-rolls a retry-with-backoff loop that duplicates
`http::retry`. It's longer, drops the jitter the shared helper applies, and is a
second place to fix bugs. Call `http::retry` instead.

### Callback splits auth logic across two modules — fidelity/minor

The design puts all of the exchange in `callback`, but error mapping leaked into
`mod.rs`. Minor, but it erodes the single-module boundary the design chose. Fold
it back.

## Verdict

Mid-grade: the flow is sound and matches the design, but the inverted state check
is a foundational bug and the duplicated retry loop is the kind of debt that
compounds. Resolve both before accepting; the boundary nit is optional.
```
