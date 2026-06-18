# Workflow templates and spec schema

Update the embedded agent templates and the spec schema doc so the audit/sync/archive flow
and the pending-vs-error distinction are explicit.

## Context

All edits are under `crates/duckspec/content/`. The intended flow: run `ds audit <change>`
after each `/ds-apply` as a progress check (pending = later steps, expected; error = this
step's checked task is unlinked, fix before handoff); a scoped audit with no errors and no
pending means archive-ready; after `ds archive`, run `ds sync` then full `ds audit`.

## Tasks

- [x] 1. `templates/apply.md` — after the `ds check <step-file>` step, add running
         `ds audit <change>` as a progress check, explaining pending vs error, and wire it
         into the existing "more steps / all done" handoff branches.

- [x] 2. `templates/step.md` — note that the change-scoped audit belongs to `/ds-apply`,
         not the step phase.

- [x] 3. `templates/archive.md` — after `ds archive <name>`, instruct `ds sync` (stamp
         backlinks into the freshly-landed caps specs) then full `ds audit`.

- [x] 4. `templates/verify.md` — distinguish full `ds audit` (in-flight-tolerant) from
         `ds audit <change>` (per-change progress/completeness gate).

- [x] 5. `schemas/spec.md` — document the `test: code` marker, that source `@spec`
         comments are the backlinks the audit resolves, and that `ds sync` stamps the
         resolved `path:line` into caps markers.
