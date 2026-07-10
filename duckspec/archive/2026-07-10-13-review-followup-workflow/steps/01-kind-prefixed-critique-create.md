# Kind-prefixed critique create

Add kind-aware critique creation in duckpond and the CLI so review and followup share the
append-only `reviews/` sequence with `review-` / `followup-` slug prefixes.

## Tasks

- [x] 1. In `crates/duckpond/src/plan.rs`, add `CritiqueKind { Review, Followup }` with
         `as_str`, implement `create_critique(name, kind, change, active, existing)` that
         slugifies the title, prefixes once with the kind, appends next `NN`, and rejects
         empty title slug or full-slug conflicts; wire `create_review` through
         `CritiqueKind::Review` (or replace call sites)

- [x] 2. Add `followup` to `STAGES` in `plan.rs` so hooks can target the stage

- [x] 3. Update existing `create_review` unit tests for kind-prefixed paths
         (`01-review-…`) and add tests for followup create, shared sequence, and dual kind
         prefixes

- [x] 4. @spec review Sequential numbering: A followup continues the shared sequence after a review

- [x] 5. @spec review Sequential numbering: Review and followup with the same title portion both create

- [x] 6. @spec review Filename slug: A followup create prefixes the slug with followup-

- [x] 7. In `crates/duckspec/src/cmd/create.rs`, add
         `CreateCommand::Followup { name, change }`, call `create_critique` with
         `Followup`, and seed placeholder `# Followup\n` (mirror Review)

- [x] 8. Run `cargo test -p duckpond` for plan/review tests and fix any path assertions
         that still expect unprefixed review slugs
