# Review schema

A review is the durable synthesis of an agent-led inspection and the
finding-by-finding discussion that followed. It records evidence, important
trade-offs, the agreed resolution, and the earliest corrective stage so a new
session can act without reconstructing the conversation.

## Structure

```markdown
# <Review title>

<compact outcome summary>

## Scope

<artifacts, source, tests, and stage examined>

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

- Path: `duckspec/changes/<name>/reviews/NN-review-<slug>.md`
- H1 title required; non-empty summary paragraph follows it
- Body is freeform markdown; Structure is the expected durable shape
- Summary rows and Findings headings use the same numbering
- Every accepted finding has a resolved conclusion and one earliest next stage
- Valid finding routes: `/ds-design`, `/ds-spec`, `/ds-step`
- The record contains no unresolved findings or open questions
- Append-only log: create a new file; do not renumber or rewrite history

## Quality

- **Full decision context.** Evidence, impact, important alternatives, agreed
  resolution, and specific next work let a cold stage act without redoing the
  review conversation.
- **Grounded.** Exact project evidence supports every finding. Mechanical
  failures already owned by checks are reported by those checks, not padded
  into review prose.
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
# Review: OAuth callback

The callback flow needs a design amendment before its contract or code can be
accepted; session reuse remains sound.

## Scope

Reviewed the OAuth proposal, design, capability pair, callback implementation,
and integration tests after implementation.

## Summary

| # | finding | resolution | → next |
| --- | --- | --- | --- |
| 1 | Account linking has two owners | Centralize linking in the identity service | /ds-design |

## Findings

### 1. Account linking has two owners

**Where:** `design.md` account-linking flow; `src/oauth/callback.rs:70`

**Evidence:** The callback creates links directly while the identity service
also owns uniqueness and conflict handling.

**Impact:** Two write paths can apply different conflict rules and drift.

**Discussion:** Keeping callback ownership is locally smaller, but duplicates
the identity invariant. Routing all links through the identity service adds one
call and preserves a single authority.

**Resolution:** The identity service owns link creation and conflicts; the
callback only supplies the verified provider identity.

**Next:** `/ds-design` - amend ownership and callback flow before revising specs.

## Resolved concerns

Opaque session reuse was rechecked and remains intentional; OAuth converges on
the existing session boundary.

## Outcome

Not ready to freeze. Amend the design first; spec and implementation work
follow from that settled direction.
```
