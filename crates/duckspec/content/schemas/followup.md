# Followup schema

A followup is the durable synthesis of user-led concerns, the investigation
they prompted, and the finding-by-finding discussion that followed. It records
evidence, important trade-offs, the agreed resolution, and the earliest
corrective stage so a new session can act without reconstructing the
conversation.

## Structure

```markdown
# <Followup title>

<compact outcome summary>

## Scope

<concerns, artifacts, source, tests, and stage examined>

## Summary

| # | finding | resolution | → next |
| --- | --- | --- | --- |
| 1 | <short title> | <agreed conclusion> | /ds-design |

## Findings

### 1. <Finding title>

**Where:** <paths, lines, artifacts, or sections>

**Evidence:** <what was observed>

**Impact:** <why it matters if unchanged>

**Discussion:** <material alternatives and trade-offs considered>

**Resolution:** <agreed conclusion>

**Next:** </ds-design, /ds-spec, or /ds-step> - <specific work the stage can act on>

## Resolved concerns

<optional dismissed candidates whose resolution is durable>

## Outcome

<aggregate readiness and primary next route>
```

`Resolved concerns` is optional. Empty Summary is valid when no accepted
findings remain; Outcome still states readiness.

## Rules

- Path: `duckspec/changes/<name>/reviews/NN-followup-<slug>.md`
- H1 title required; non-empty summary paragraph follows it
- Body is freeform markdown; Structure is the expected durable shape
- Summary rows and Findings headings use the same numbering
- Every accepted finding has a resolved conclusion and one earliest next stage
- Valid finding routes: `/ds-design`, `/ds-spec`, `/ds-step`
- The record contains no unresolved findings or open questions
- Append-only log: create a new file; do not renumber or rewrite history

## Quality

- **User-led, evidence-grounded.** Start from the concerns the user raised, then
  preserve what inspection and discussion established rather than recording
  the initial impression as fact.
- **Full decision context.** Evidence, impact, important alternatives, agreed
  resolution, and specific next work let a cold stage act without redoing the
  followup conversation.
- **Upstream routing.** Route to design when direction is invalid, spec when
  design is sound but behavior is wrong, and step when design and contracts are
  sound but implementation needs work.
- **One finding, one issue.** Merge duplicate symptoms and keep related
  evidence together. Order findings from the earliest affected layer.
- **Resolved concerns sparingly.** Omit false leads unless the dismissal
  captures a durable intentional trade-off or prevents repeated investigation.
- **Cohesive record.** Summary supports scanning; Findings preserve reasoning;
  Outcome states the aggregate readiness without repeating every row.
- Body markdown follows `style` (load only if not already in context).

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load
only if not already in context.

## Example

```markdown
# Followup: transcript collapse timing

The user's collapse concern is valid, but the design and contract already say
the right thing; only the implementation needs correction.

## Scope

Investigated Thinking collapse across the user's report, transcript design,
capability spec, implementation, and streaming tests.

## Summary

| # | finding | resolution | → next |
| --- | --- | --- | --- |
| 1 | Thinking collapses on tool start | Code diverges from settled contract | /ds-step |

## Findings

### 1. Thinking collapses on tool start

**Where:** `src/transcript.rs:142`; collapse-default scenarios

**Evidence:** The implementation closes Thinking on the first tool event. The
design and spec both keep it open until Answer or TurnComplete.

**Impact:** Reasoning disappears while tool activity is still streaming,
contrary to the intended calm transcript.

**Discussion:** Changing the contract to tool-start collapse would reduce open
content sooner but recreate the abrupt transition the design rejected. The
existing Answer-or-completion boundary remains clearer.

**Resolution:** Keep the design and spec unchanged; correct the implementation.

**Next:** `/ds-step` - plan the collapse-trigger fix and regression coverage.

## Outcome

Design and contract are sound. The change is not archive-ready until the code
follows them.
```
