# Review classification and validation

Teach `duckpond` to recognize a review file by its path and validate it against the
document schema, in both active and archived changes.

## Tasks

- [x] 1. Add a `Review` variant to `ArtifactKind` in `crates/duckpond/src/layout.rs` and
         route the `reviews/` subdirectory through a new `classify_review` helper in
         `classify_change`, mirroring `classify_step` (single `.md` child, no nesting).

- [x] 2. Generalize `extract_step_slug` to a neutral `extract_nn_slug` in `layout.rs` and
         update its existing step callers; keep the `NN-<slug>.md` parsing behavior
         identical.

- [x] 3. Add `ArtifactKind::Review` to the doc-parsing arm in
         `crates/duckpond/src/check.rs` so reviews validate via
         `parse::doc::parse_document`.

- [x] 4. @spec review Review recognition and validation: A well-formed review validates

- [x] 5. @spec review Review recognition and validation: A review missing its H1 title is reported as a document error

- [x] 6. @spec review Review recognition and validation: A review in an archived change is still recognized
