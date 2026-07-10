# Align document-first critique write gate

Make proposal, design, and review doc match the shipped document-first critique contract:
history artifact first; plan amend only on explicit post-doc request or via later stages.

## Prerequisites

- [x] @step scannable-templates-and-schemas
- [x] @step restore-stable-chrome-scenario-names

## Context

Addresses finding 2 in `reviews/03-review-post-implementation-dual-critique.md`. Chosen
contract: document-first (align plan layers to templates), not restore in-stage plan
amend.

Templates already teach document-only write gates with optional explicit in-place fix
after the document. Update upstream artifacts that still promise in-flow amend of
proposal/design/caps/steps as part of the critique stage.

## Tasks

- [x] 1. Update `duckspec/changes/review-followup-workflow/proposal.md` so dual modes
         find/record issues and hand off rework; drop or reword claims that critique
         stages themselves amend the plan chain as part of their normal spine

- [x] 2. Update `duckspec/changes/review-followup-workflow/design.md` shared write-gate
         spine and any amend-in-session wording to document-first: create + write critique
         file; optional plan edits only when the user explicitly asks after the document
         (out of band); product code still never in critique stages

- [x] 3. Update `duckspec/changes/review-followup-workflow/caps/review/doc.delta.md` so
         review/followup describe recording judgment and recommended next steps, not
         in-stage plan amendment as default behavior

- [x] 4. Audit `crates/duckspec/content/templates/review.md`, `templates/followup.md`,
         `schemas/review.md`, and `schemas/followup.md` for leftover “amend plan in this
         stage” wording; leave them if already document-first, patch only contradictions

- [x] 5. Align step 03 summary/tasks text if it still advertises “optional plan-amend
         write gate” as in-stage amend, so historical steps do not contradict the chosen
         contract
