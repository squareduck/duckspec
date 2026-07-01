# Unify duckboard idea slug

Delete duckboard's private `slugify` and route idea filenames through
`duckpond::slug::slugify`, keeping the `"idea"` fallback at the call site.

## Prerequisites

- [x] @step extract-the-slug-module

## Tasks

- [x] 1. Delete `idea_store::slugify` at `idea_store.rs:163`; repoint the title site
         (`idea_store.rs:425`) to `duckpond::slug::slugify`, substituting `"idea"` when
         the result is empty.

- [x] 2. Repoint the tag-segment site in `primary_tag_segments` (`idea_store.rs:186`) to
         `duckpond::slug::slugify`, filtering out segments that slugify to empty.

- [x] 3. Update the `idea_store` slug unit tests for the new behavior: Unicode
         alphanumerics preserved, and the `"idea"` fallback verified at the title call
         site rather than inside `slugify`.

- [x] 4. Run `cargo test -p duckboard` and confirm it passes.
