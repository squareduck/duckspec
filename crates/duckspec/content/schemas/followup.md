# Followup schema

A followup is a **user-led critique record** on a change: same purpose as a
review (issues, judgment, recommended next steps), sourced from conversation
rather than a solo agent scan. Append-only under
`duckspec/changes/<name>/reviews/NN-followup-<slug>.md` (shared log with
reviews).

## Structure

```markdown
# <Followup Title>

<1-2 sentence summary: discussion, stage, headline outcome>

## Scope

<artifacts examined; stage; deepest layer discussed>

## Summary

| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | quality | <short title> | /ds-step |
| 2 | minor | fidelity | <short title> | /ds-spec |

## Issues

### 1. <Issue title> - <lens>/<severity>

**Where:** <`path:line` or artifact + section>

**Why:** <why it matters>

**Action:** <recommended stage or approach - not work already done>

## Open questions

<genuine unresolved decisions. Omit if none.>

## Outcome

<what was agreed; possible next moves; archive-readiness. Do not claim
plan/code changed unless that happened outside this write.>
```

```
proposal --> design --> caps (spec/doc) --> code
```

May cover any layer the user cares about. Number Summary rows and Issues
headings the same. Column `→ next` is a stage or path - not "already fixed".

## Lenses and severity

Same as **review** (`ds schema review`): soundness, fidelity, quality; critical /
major / minor by lasting harm if frozen as-is.

## Rules

- Path: `duckspec/changes/<name>/reviews/NN-followup-<slug>.md` (`followup-`
  prefix on create)
- H1 title required; non-empty summary paragraph follows it
- Body freeform; Structure is recommended (document schema only for `ds check`)
- Append-only log - new file per followup; do not renumber or rewrite history

## Quality

- **Scannable first.** Triage from Summary; depth under Where / Why / Action
- **User-led.** Record what the human raised and agreed - not a performative
  solo re-review of settled choices
- **Recommend, don't apply.** Action is next path, not a log of in-session edits
- **Don't re-verify** what check/audit already prove
- **Issues are actionable.** Observations without a path are prose, not rows
- Body markdown follows `style` (load only if not already in context)

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

```markdown
# Followup: collapse policy

User-led pass after review: collapse should wait for Answer / TurnComplete, not
tool start.

## Scope

Post-implementation followup on `chat-calm-transcript`: design collapse table,
`caps/chat/transcript`, and the open review finding on Thinking collapse.

## Summary

| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | critical | soundness | Thinking collapses on tool start | /ds-step |

## Issues

### 1. Thinking collapses on tool start - soundness/critical

**Where:** design collapse table; `caps/chat/transcript` Collapse defaults

**Why:** Live Thinking should stay open through tools until Answer or
TurnComplete; early collapse breaks the calm UX.

**Action:** Retarget triggers to Answer / TurnComplete; fix via `/ds-step` /
`/ds-apply` (and `/ds-spec` if the cap is not updated yet).

## Outcome

Agreed on the collapse contract. Plan and code unchanged in this session.
Suggested next: `/ds-step`. Not archive-ready until the fix lands.
```
