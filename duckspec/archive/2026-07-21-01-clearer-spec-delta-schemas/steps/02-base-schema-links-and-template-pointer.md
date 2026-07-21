# Base schema links and template pointer

Point base `spec` / `doc` schemas at deltas for shape; keep `/ds-spec` process-only so
marker judgment lives only in the delta schemas.

## Prerequisites

- [x] @step expand-delta-schemas

## Tasks

- [x] 1. Add a minimal Rules/post-Rules cross-link in
         `crates/duckspec/content/schemas/spec.md`: merged and delta-authored bodies still
         obey this schema; delta shape is `ds schema spec-delta` (no marker grammar)

- [x] 2. Same cross-link pattern in `crates/duckspec/content/schemas/doc.md` pointing at
         `ds schema doc-delta`

- [x] 3. Edit `crates/duckspec/content/templates/spec.md` “On disk after each confirm”
         UPDATE bullet: write deltas per schemas; remove parenthetical lightest-touch /
         `@`+`+` / stable-title judgment (now schema-owned)

- [x] 4. Confirm template still loads schemas before draft and does not restate marker
         tables or multi-marker examples
