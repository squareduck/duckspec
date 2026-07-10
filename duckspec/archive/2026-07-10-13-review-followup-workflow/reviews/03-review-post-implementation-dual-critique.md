# Post-implementation dual critique

End-to-end review of `review-followup-workflow` after all steps. Dual critique,
kind-prefixed create, and chrome arms largely land; project audit and plan-amend
write-gate alignment do not.

## Scope

Post-implementation review of change `review-followup-workflow`: proposal, design, caps
deltas (`review`, `chat/obvious-bubble`, `session/scope`), steps 01–03, prior followups
under `reviews/`, and shipped work in duckpond plan create, duckboard
`change_scope_facts`, duckspec content (schemas/templates/commands), and duckchat
reply-suggest. Deepest layer: code and embedded content.

## Summary

```
| # | sev | lens | title | → next |
|---|-----|------|-------|--------|
| 1 | critical | soundness | Scenario renames break project audit coverage | /ds-step |
| 2 | major | fidelity | Plan-amend write gate disagrees across layers | /ds-step |
| 3 | minor | quality | Stale scenario titles and unprefixed step example | /ds-step |
```

## Findings

### 1. Scenario renames break project audit coverage — soundness/critical

**Where:**
`duckspec/changes/review-followup-workflow/caps/chat/obvious-bubble/spec.delta.md` (`=`
renames for open-steps-with-review and no-open-steps-with-review);
`crates/duckboard/src/area/change.rs` `@spec` markers retargeted to the new names;
archived
`duckspec/archive/2026-07-10-09-bubble-create-change-and-design-step/steps/03-gate-steps-and-review-aware-ladder.md`
still has checked `@spec` tasks on the **old** names.

**Why:** `ds audit` (project) currently fails with two errors: the old scenario names
still exist on the top-level cap, have checked step tasks in archive history, and no
longer have resolving test backlinks (tests moved to the renamed titles that only exist in
the active delta). Archiving this change will rename the live scenarios away and leave
those archived `@spec` lines permanently orphaned. Project principle is that spec drift is
a build-time error — a green change-scoped audit is not enough if full project audit is
red and will stay red after freeze.

**Action:** Prefer stable scenario titles with body-only `~` replaces (and keep `@spec`
strings matching), **or** deliberately rename and also repair historical checked `@spec`
lines / coverage so project audit is clean before archive. Re-run `ds audit` with no
change filter until both errors clear.

### 2. Plan-amend write gate disagrees across layers — fidelity/major

**Where:** proposal (both modes “may amend” plan chain); design shared write-gate spine
step 4 (optional amend of proposal/design/caps/steps after create); review doc delta
(“Either kind may note amendments…”); vs shipped
`crates/duckspec/content/templates/review.md` and `followup.md` write gates
(document-only; plan edits only if the user explicitly asks **after** the document,
outside the stage spine); schemas’ Action guidance (“recommended next… not work already
performed”).

**Why:** Followup’s stated reason for existence is user-led course correction that can
fold corrections into the plan while preserving history. Templates still allow an explicit
post-doc fix, but agent instructions treat plan amend as out of band. Cap/doc and design
still describe in-flow amend. Agents and humans will disagree about whether amending
design/caps inside `/ds-review` or `/ds-followup` is correct — durable process debt if
frozen.

**Action:** Pick one contract and align all layers: either restore optional in-stage plan
amend (design spine) in both templates and schemas, **or** update proposal/design/review
doc to the document-first model (record issues; rework via `/ds-spec` / `/ds-step` /
explicit in-place ask only). Do not leave cap prose and templates contradictory.

### 3. Stale scenario titles and unprefixed step example — quality/minor

**Where:**

- Cap scenario title still `All steps complete yield archive then review` while THEN lists
  archive, review, **and** followup (base + delta `~` bodies; `change.rs` `@spec` keeps
  the old title).

- `crates/duckspec/content/templates/step.md` example cite
  `reviews/02-post-implementation.md` (no kind prefix) though new creates always write
  `NN-review-…` / `NN-followup-…`.

**Why:** Low cost, but teaches the wrong filename shape and leaves scenario names that no
longer match behavior — small compounding confusion for agents reading caps and templates.

**Action:** Rename or reword the all-done scenario title to mention followup (if keeping
renames after finding 1’s policy); update the step template example to a kind-prefixed
path such as `reviews/02-review-post-implementation.md`.

## Verdict

The architecture is sound: one `reviews/` log, `CritiqueKind` + `create_critique`, peer
schemas/templates, and lifecycle arms that keep both critique modes available match the
proposal. Implementation quality of plan create and chrome is simple and well-tested. Not
ready to accept or archive until project audit is green again (finding 1) and the amend
write-gate story is one coherent contract (finding 2). Finding 3 is polish after those.
