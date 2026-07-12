# Review polish residual examples

Align residual examples with decision-named tokens and the reason split from the
post-implementation review.

## Context

From `reviews/01-review-post-implementation-concrete-gate-tokens.md`: finding 1
(meta-cards doc bare `confirm`) and finding 2 (archive `commit` reason).

## Tasks

- [x] 1. Update `duckspec/caps/chat/meta-cards/doc.md` examples: decision-named tokens
         instead of bare `confirm` / `reject`; use a slash line with a reason if
         demonstrating optional reasons

- [x] 2. Drop the reason on the archive handoff chip in
         `crates/duckspec/content/templates/archive.md` (`` `commit` `` alone)

- [x] 3. `ds format` / `ds check` touched paths
