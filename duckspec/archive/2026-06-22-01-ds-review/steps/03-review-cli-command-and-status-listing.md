# Review CLI command and status listing

Add the `ds create review` command and surface reviews in `ds status`.

## Prerequisites

- [ ] @step review-creation-planner

## Context

Steps and reviews share the `NN-<slug>.md` filename shape, so `placeholder_for(filename)`
in `cmd/create.rs` cannot tell them apart to emit `# Review` vs `# Step`. Thread the
placeholder title from the command (which knows it is creating a review) rather than
inferring it from the filename — see the design's "ds create review command" section.

## Tasks

- [x] 1. Add a `Review { name, #[arg(long = "in")] change }` variant to `CreateCommand` in
         `crates/duckspec/src/cmd/create.rs`, and dispatch it: list
         `changes/<change>/reviews/` and call `duckpond::plan::create_review`.

- [x] 2. Thread an explicit placeholder title through the create path so a freshly created
         review file is seeded with `# Review`, without relying on filename sniffing.

- [x] 3. Add a `Review` arm to `summarize_change` in `crates/duckspec/src/cmd/status.rs`
         so the project-level status counts a change's reviews.

- [x] 4. Add a `Review` arm to `status_change` in `cmd/status.rs` so the per-change status
         lists the reviews.
