# ds-review schema, template, and commands

Author the agent-facing content for the review stage: the schema guidance, the
two-movement template, and the harness command files.

## Context

These are data-driven content files — no Rust wiring beyond the `STAGES` entry.
`ds schema review`, `ds template review`, and `ds init` each read their content directory
by name, so dropping the files in is enough.

Mirror the shape of existing siblings:

- schema → `content/schemas/proposal.md` (prose guidance for a doc-parser artifact:
  conventional sections + quality notes; no GWT, no parser).

- template → `content/templates/verify.md` (the standard `## Before write` …
  `## After write` skeleton with `## Role` / `## Voice` / `## Context` / `## Instructions`
  / `## Write gate` / `## Handoff`).

- commands → `content/commands/claude/ds-explore.md` and the `opencode/` counterpart.

The template has **two movements**: (1) produce the next `reviews/NN-<slug>.md` via
`ds create review`, critiquing the change against its contract and the diff from a fresh,
adversarial stance — advisory only, never gating; (2) only when the user is ready,
generate review-sourced fix-steps (mirroring `/ds-step`) whose `## Context` cites the
originating review. The schema file defines the review document's conventional shape
(findings with severity, recommended actions, verdict) and is what the template tells the
agent to load.

## Tasks

- [x] 1. Write `content/schemas/review.md` — the review document's guidance: conventional
         structure (findings with severity, scope, recommended actions, verdict) and
         quality notes, in the style of `content/schemas/proposal.md`.

- [x] 2. Write `content/templates/review.md` — the `/ds-review` template in the standard
         skeleton, with the two movements described in Context; reference
         `ds schema review` in the first movement.

- [x] 3. Write `content/commands/claude/ds-review.md`, mirroring the existing `ds-*`
         command files (run `ds template review` silently, then follow its instructions).

- [x] 4. Write `content/commands/opencode/ds-review.md`, the opencode counterpart.

- [x] 5. Add `"review"` to the `STAGES` constant in `crates/duckpond/src/plan.rs` so
         `ds create hook review --pre/--post` works.
