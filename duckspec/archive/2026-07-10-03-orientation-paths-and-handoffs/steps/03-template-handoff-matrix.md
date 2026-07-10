# Template handoff matrix

Rewrite workflow template Handoff sections to a hard ranked ≤2 next-action matrix matching
the design (review before archive, commit after archive).

## Tasks

- [x] 1. Update Handoffs in
         `crates/duckspec/content/templates/{explore,propose,design,spec,step,apply,archive}.md`
         to the ranked matrix from the design (explore: create change / propose; propose:
         design / spec; design: resolve open questions or spec / step; spec: step /
         archive; step: apply only; apply open: apply only; apply done: review / archive;
         archive: commit with proposed message and wait).

- [x] 2. State the shared rule explicitly where helpful: at most two ranked suggestions (①
         primary, ② secondary if any); offer once; operational notes are not next-stage
         slots.

- [x] 3. Align `codex.md`, `backfill.md`, and `review.md` handoffs with ≤2 (codex: resume
         stage or nothing; backfill: propose; review: single next by finding type).

- [x] 4. Ensure no main-flow Handoff suggests `/ds-verify`.

- [x] 5. Spot-check that write-gate / Context path strings in those templates already use
         `duckspec/…` and leave them unless a Handoff edit forces a consistency fix.
