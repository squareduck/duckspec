# Followup schema

A followup is a **user-led critique record** on a change — the same purpose as a
review (find issues; record judgment and recommended next steps), driven by
conversation rather than a solo agent scan. It is stored under the change's
append-only `reviews/` log: `changes/<name>/reviews/NN-followup-<slug>.md`.

Static tooling (`ds check`, and change-scoped `ds audit <change>` elsewhere)
owns well-*formed*; a followup captures course correction the human wants
preserved and does not run audit. **Producing this document is the whole
job of `/ds-followup`.** Applying fixes (plan, code, or templates) is a later
choice by the user — `/ds-spec`, `/ds-step`, ignore, or an explicit in-place
request — not part of writing the followup.

## What a followup covers

A change is a chain — each layer is built on the one above it:

```
proposal ──→ design ──→ caps (spec/doc) ──→ code
```

A followup may discuss any layer the user cares about at the current stage. It
uses the same judgment lenses as a review when tagging issues:

- **soundness** — is this artifact, on its own terms, *right*?
- **fidelity** — does each layer faithfully realize the one above it?
- **quality** — is it well-*made*?

Unlike a review, issues come from dialogue with the user. The agent helps
sharpen, locate, and **record** them. It does not amend proposal, design, caps,
steps, or product code as part of creating the followup.

## Structure

Write for two read modes: a **Summary** table for triage, then structured detail
under **Issues**. Chat presentation should match the Summary table.

```markdown
# <Followup Title>

<1-2 sentence summary: what was discussed, at what stage, and the headline outcome>

## Scope

<what this followup covers — the artifacts examined, and the stage the change is
at. Name the deepest layer discussed.>

## Summary

| # | sev | lens | title | → next |
|---|-----|------|-------|--------|
| 1 | major | quality | Soft-wrap duplicated in table cells | /ds-step |
| 2 | minor | fidelity | Proposal Impact omits open-questions arm | /ds-spec |

## Issues

### 1. <Issue title> — <lens>/<severity>

**Where:** <`path:line` or artifact + section>

**Why:** <why it matters>

**Action:** <recommended approach or next stage — not work already performed in
this session>

## Open questions

<genuine unresolved decisions. Omit if none.>

## Outcome

<aggregate outcome: what was agreed, what the user might do next, whether the
change looks closer to archive-ready. Do not claim plan/code was changed unless
that happened outside this document write.>
```

Number issues in Summary and reuse those numbers in Issues headings. The `→ next`
column recommends a stage or path (`/ds-spec`, `/ds-step`, `/ds-archive`, or
`ignore` / discussion) — not an "already amended" status for in-session edits.

## Severity

Same scale as reviews — lasting harm if frozen as-is, independent of lens:

- **critical** — lasting structural harm; address before accepting the change as
  done.
- **major** — real durable drag if frozen.
- **minor** — low-cost polish that does not compound.

## Rules

- H1 title is required.
- A summary paragraph directly follows the H1.
- The body is freeform markdown — the sections above are recommended, not enforced
  by `ds check`. A followup validates against the document schema only (same
  recognition as any file under `reviews/`).
- New creates use a `followup-` slug prefix (`NN-followup-<slug>.md`).

## Quality

- **Document first.** The followup file is the deliverable; everything else is
  out of band.
- **Scannable first.** Triage from Summary; depth under **Where** / **Why** /
  **Action**.
- **User-led, not ceremonial.** Record what the human decided, not a performative
  solo critique that re-litigates settled choices.
- **Recommend, don't apply.** Action describes what should happen next; it does
  not narrate edits performed during `/ds-followup`.
- **Don't re-verify** what `ds check` (or a prior change-scoped audit) already
  prove — followup does not run audit.
- **An issue is actionable.** Observations without a recommended path are prose,
  not table rows.

## Formatting

After writing or updating this artifact, run `ds format <path>` to apply canonical
formatting (line wrap, indentation, blank lines).

Use fenced code blocks for tables and diagrams; add a `<language>` tag to fences
that contain real code.

## Example

```markdown
# Followup: collapse policy

User-led pass on `chat-calm-transcript` after review: collapse should wait for
Answer / TurnComplete, not tool start.

## Scope

Post-implementation followup on change `chat-calm-transcript`: design collapse
table, `caps/chat/transcript`, and the open review finding on Thinking collapse.

## Summary

| # | sev | lens | title | → next |
|---|-----|------|-------|--------|
| 1 | critical | soundness | Thinking collapses on tool start | /ds-step |

## Issues

### 1. Thinking collapses on tool start — soundness/critical

**Where:** design collapse table; `caps/chat/transcript` Collapse defaults

**Why:** Live Thinking should stay open through tools until Answer or
TurnComplete; collapsing early breaks the calm UX the change exists for.

**Action:** Retarget design + cap triggers to Answer / TurnComplete only, then
fix implementation and tests via `/ds-step` / `/ds-apply`.

## Outcome

Agreed on the intended collapse contract. Plan and code were not changed in this
session. Suggested next: `/ds-step` (or `/ds-spec` if the cap delta is not yet
written). Not archive-ready until the fix lands.
```
