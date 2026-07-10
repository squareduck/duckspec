# Critique chrome title polish and step example

Polish under the stable-name policy: keep all-done scenario title stable; fix the step
template example to a kind-prefixed reviews path.

## Prerequisites

- [x] @step restore-stable-chrome-scenario-names
- [x] @step align-document-first-critique-write-gate

## Context

Addresses finding 3 in `reviews/03-review-post-implementation-dual-critique.md`. Do
**not** rename `All steps complete yield archive then review` (or other stable titles) —
that reopens finding 1. Title may under-describe followup; body and chrome lists already
include it.

## Tasks

- [x] 1. Confirm `All steps complete yield archive then review` and
         `All steps complete nonempty session includes Confirm and Reject` stay under
         stable titles with bodies that list `/ds-followup`; update doc tables only if
         they claim a shorter list without followup

- [x] 2. In `crates/duckspec/content/templates/step.md`, change the review cite example
         from `reviews/02-post-implementation.md` to a kind-prefixed path such as
         `reviews/02-review-post-implementation.md`

- [x] 3. Run `ds format` on any edited plan/content paths from steps 04–06 and re-run full
         `ds audit` to confirm still green
