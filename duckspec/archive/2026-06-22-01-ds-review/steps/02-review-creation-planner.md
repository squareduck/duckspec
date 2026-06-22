# Review creation planner

Add the append-only `create_review` planner to `duckpond`, assigning the next sequential
number and rejecting duplicate slugs.

## Prerequisites

- [ ] @step review-classification-and-validation

## Tasks

- [x] 1. In `crates/duckpond/src/plan.rs`, rename the `parse_steps` helper to
         `parse_nn_slug` (neutral, shared by steps and reviews) and add a `review_path`
         helper mirroring `step_path`.

- [x] 2. Add a `ReviewSlugExists { slug }` variant to `PlanError`.

- [x] 3. Implement `plan::create_review` mirroring the append branch of `create_step` (no
         `--after`): number the new review one above the highest existing review, reject a
         duplicate slug, and emit no renames.

- [x] 4. @spec review Sequential numbering: The first review in a change is numbered 01

- [x] 5. @spec review Sequential numbering: A new review is numbered above the highest existing review

- [x] 6. @spec review Sequential numbering: A review whose slug already exists is rejected
