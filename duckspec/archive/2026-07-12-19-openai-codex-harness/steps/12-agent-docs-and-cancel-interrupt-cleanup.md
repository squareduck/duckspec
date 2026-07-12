# Agent docs and cancel interrupt cleanup

Refresh stale agent shell docs and resolve the unused `turn_interrupt` helper versus
kill-only cancel heat.

## Prerequisites

- [x] @step multi-question-requestuserinput

## Context

From review finding 4
(`reviews/02-review-post-implementation-review-of-openai-codex-harness.md`).

Prefer tracking an in-flight turn id and calling `turn/interrupt` before kill on cancel.
If that is awkward, remove dead `turn_interrupt` and document kill-only heat end in the
harness doc (host already kills the ACP child, Claude pattern).

## Tasks

- [x] 1. Rewrite `crates/duckchat-codex-acp/src/main.rs` crate docs to describe the live
         App Server bridge (remove “Backend wiring lands in later steps”)

- [x] 2. Either track in-flight turn id and call `turn/interrupt` before kill on cancel,
         or remove dead `turn_interrupt` and document kill-only heat end in
         `caps/harness/openai-codex/doc.md` Session lifecycle

- [x] 3. @spec harness/openai-codex App-server process heat: After cancel, a later turn may spawn again and resume a prior session id
