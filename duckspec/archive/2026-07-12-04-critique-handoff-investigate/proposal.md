# Critique handoff investigate

Tighten post-write review and followup handoffs so next actions match readiness and
ranking, and drop noop chips that invite inaction.

## Motivation

After a review or followup lands on disk, agents too often hand off straight to `/ds-step`
even when the real gap is missing or wrong observable behavior (which belongs in specs
first). They also offer stage commands when the fix path is still unclear, so the next
stage cannot act without re-deriving the conversation.

Separately, trailing `next` meta cards sometimes include noop options like `ignore`. Those
do nothing: the user either picks a real action or types their own message. A chip that
means “do nothing” is dead weight.

Why now: review and followup already produce solid records; the daily friction is the
handoff after the write, not the report itself. A small template/`style` fix prevents bad
next-stage defaults without redesigning critique.

## Intent

- After a clean review or followup write, the trailing `next` meta card ranks real next
  work only

- When the recorded issues still need a clearer fix path before a stage could act cold,
  the primary offer is the bare token `investigate` (continue in chat)

- When behavior or invariants need capture, `/ds-spec` is primary and `/ds-step` is
  available second so the user can skip capture deliberately

- When specs already cover the fix (or work is pure rework), `/ds-step` alone is
  appropriate

- Archive remains available when the change is freeze-ready

- Noop handoff tokens (including `ignore`) are not offered; if nothing useful applies, the
  `next` meta card is omitted and the user types freeform

- The report may still record an incomplete solution picture; readiness applies only to
  which handoff actions are offered

## Non-goals

- Changing how review/followup documents are structured or when their write gates fire

- Reworking lifecycle chrome, orientation ladders, or other stages’ handoffs beyond
  removing shared noop-token teaching in `style`

- Product-code or duckboard behavior for bare tokens beyond existing `next` meta card send
  behavior

- Migrating historical reviews that used `ignore` as a disk `→ next` label
