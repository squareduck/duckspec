# Reject empty slugs in ds create

Repoint step and review creation at the shared slug rule, delete the buggy local
`slugify`, and reject a title that slugifies to empty.

## Prerequisites

- [x] @step extract-the-slug-module

## Tasks

- [x] 1. Add `PlanError::EmptySlug { title: String }` to `plan.rs` with a message naming
         the offending title.

- [x] 2. Delete the local `slugify` at `plan.rs:393`; repoint `create_step` and
         `create_review` at `crate::slug::slugify`, rejecting an empty slug with
         `PlanError::EmptySlug` before the uniqueness and numbering logic.

- [x] 3. Remove the now-redundant `slugify` unit tests in `plan.rs` (covered by
         `slug.rs`), and add a unit test asserting `create_step` rejects an
         all-non-alphanumeric title with `EmptySlug`.

- [x] 4. @spec review Filename slug: A punctuated title produces a dash-normalized slug

- [x] 5. @spec review Filename slug: A title with no alphanumeric characters is rejected
