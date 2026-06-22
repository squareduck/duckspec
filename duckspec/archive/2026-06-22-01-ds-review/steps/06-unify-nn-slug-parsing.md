# Unify nn slug parsing

Collapse the parallel `NN-<slug>.md` parsers onto a single canonical helper in
`duckpond::layout`, and have duckboard select the current review by parsed number rather
than by lexicographic sort order.

## Context

Addresses the "`build_reviews` re-rolls a looser NN parser" finding in
`reviews/01-post-implementation.md`. Today three parsers coexist:
`duckpond::layout::extract_nn_slug` (returns only the slug, strict two-digit prefix),
`duckpond::plan`'s private `parse_nn_slug` / `ParsedNnSlug` (returns number + slug,
strict), and duckboard's inline loops in `build_steps` / `build_reviews`
(`crates/duckboard/src/data.rs`) which accept any digit count via
`num_str.parse::<u32>().is_ok()`. The duckboard variant is looser than the canonical rule
and relies on `read_sorted_dir`'s lexicographic order equalling numeric order, so
`change.reviews.last()` (`area/change.rs`) can pick the wrong "current" review for
non-canonical or 100+ names.

This is plumbing — the specced orientation behavior is unchanged and already covered by
tests in `area/change.rs`; this step only removes duplication and makes "highest-numbered"
robust. No new `@spec` tasks.

## Tasks

- [x] 1. In `crates/duckpond/src/layout.rs`, add a canonical parser (e.g. `parse_nn_slug`)
         that returns both the number and slug from an `NN-<slug>.md` filename under the
         strict two-digit-prefix rule. Reimplement the existing `extract_nn_slug` to
         delegate to it and return only the slug, keeping its behavior and doctest
         identical.

- [x] 2. Route `crates/duckpond/src/plan.rs` through the canonical helper: have its
         `ParsedNnSlug` / `parse_nn_slug` (the slice version) build on the new `layout`
         parser instead of re-deriving the split, so there is one source of truth for the
         two-digit rule.

- [x] 3. In `crates/duckboard/src/data.rs`, rewrite `build_steps` and `build_reviews` to
         filter via the canonical `layout` parser and sort by the parsed number ascending,
         so the last entry is genuinely the highest-numbered. Drop the inline
         `num_str.parse::<u32>().is_ok()` loops.

- [x] 4. Confirm `crates/duckboard/src/area/change.rs` still derives `current_review`
         correctly via `change.reviews.last()` now that `build_reviews` sorts by parsed
         number; switch to an explicit max-by-number if `.last()` is no longer the right
         selector.

- [x] 5. Extend the `layout.rs` unit tests to cover the canonical parser's number+slug
         output (basic, multi-segment slug, single- and three-digit prefixes rejected),
         and run `cargo test` to confirm the whole suite — including the orientation tests
         in `area/change.rs` — stays green.
