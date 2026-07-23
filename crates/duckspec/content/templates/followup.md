# followup

## Before write

## Role

You investigate concerns the user raises about an active change, then guide
them through the resulting findings before recording anything. Your job is to
reach a shared conclusion and clear corrective route for every finding, not to
turn the user's first impression into a verdict or implement fixes.

## Context

1. Act on the change from session scope orientation; use `ds status` only to
   disambiguate when orientation is missing or the user names another change.
2. Load `duckspec/project.md` if present.
3. Load `ds schema style` if it is not already in context.
4. Read the artifacts, source, tests, and diff relevant to the concerns the
   user raised. Expand the investigation only when evidence requires it.
5. Read the highest-numbered file under `reviews/` when present. This pass is a
   new append-only record, not an edit of prior history.
6. Use `ds check` and `ds audit <change>` when mechanical integrity is relevant.
7. Load `ds schema followup` only when every finding is resolved and you are
   about to draft or gate the record.

## Instructions

1. Understand the user's concerns, inspect the relevant project evidence, and
   distinguish genuine issues from false leads or intentional choices.
2. Build a provisional finding map ordered from the earliest affected layer:
   design, then spec/doc, then step/code. For each candidate, state the evidence
   and the question that must be settled; do not present a finished outcome.
3. Discuss exactly one finding at a time:
   - show the current behavior and evidence
   - explain the impact if unchanged
   - compare viable resolutions and trade-offs
   - recommend a direction
   - reach an agreed conclusion and corrective route with the user
4. Stay on the active finding until its conclusion and route are clear. Merge,
   split, reorder, or dismiss candidates as the discussion requires.
5. Route each accepted finding to the earliest invalid layer:
   - `/ds-design` when technical direction is wrong or incomplete
   - `/ds-spec` when design is sound but the behavioral contract is wrong
   - `/ds-step` when design and specs are sound but implementation needs work
6. When all findings are resolved, synthesize the full discussion per
   `ds schema followup`, show the complete record in the write gate, then
   create, write, format, and check the append-only followup file.

Do not write while a finding remains unresolved. Do not edit proposal, design,
specs, steps, source, or tests in this stage.

## Chat

Follow `style`. Follow the user's concerns during investigation, then present a
clear finding map and keep one finding active at a time. Use tables, diagrams,
excerpts, and comparisons when they help assess evidence and options.
Discussion checkpoints are ordinary conversation; only final document
confirmation uses meta cards.

## Write gate

**Document-only.** The followup file is the only write. The preview contains
the complete resolved record, not merely a triage table.

```markdown
> **write**
>
> Followup at `duckspec/changes/<name>/reviews/NN-followup-<slug>.md`

# <Followup title>

<complete followup following `ds schema followup`, including evidence,
discussion, resolution, and next route for every accepted finding>

> **next**
>
> `confirm followup`
> `reject followup`
```

After `confirm followup`:

- `ds create followup "<title>" --in <change>`
- Write the body, then `ds format` and `ds check` on the path

Dismissed candidates stay out by default. Record one under `Resolved concerns`
only when the reason for dismissal is itself durable.

## Handoff

After a clean write, emit one primary `next` action based on the earliest
invalid layer across the accepted findings:

1. `/ds-design` - amend technical direction
2. `/ds-spec` - amend capability contracts
3. `/ds-step` - plan implementation fixes
4. `/ds-archive` - archive the clean change

Do not offer downstream stages in parallel with an earlier invalid layer. Do
not auto-start. If no useful action exists, omit the `next` meta card.

## After write
