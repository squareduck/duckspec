# Guard the step-punctuation round-trip

Add a direct regression test for the headline bug: a punctuated step title must create a
filename whose slug matches what the parser derives from the H1.

## Context

Addresses the fidelity finding in `reviews/01-post-implementation-slug-unification.md`.
The proposal frames punctuated step titles as the sharp bug — before this change, creation
and validation slugs disagreed, so such a step failed `ds check` with a slug mismatch.
`create_step` and `create_review` now both call `crate::slug::slugify`, and
`review_punctuated_title_is_dash_normalized` already covers the review path, but no test
exercises the step path directly. Mirror that test for `create_step` in the `plan.rs` test
module.

## Tasks

- [x] 1. In `crates/duckpond/src/plan.rs` tests, add
         `create_step_punctuated_title_is_dash_normalized`, mirroring
         `review_punctuated_title_is_dash_normalized`: create a step from a punctuated
         title (e.g. `Fix: parser & lexer`) and assert `plan.creates` is the expected
         `changes/<change>/steps/01-fix-parser-lexer.md`.

- [x] 2. Run `cargo test -p duckpond` and confirm it passes.
