# Expand delta schemas

Rewrite `spec-delta` and `doc-delta` stock schemas so heavy remove/replace and marker
child rules match merge semantics, with one multi-marker example each.

## Tasks

- [x] 1. Expand `crates/duckspec/content/schemas/spec-delta.md` Markers table (body,
         nested headings, effect) and Structure note for content vs operations children

- [x] 2. Add Rules for `@` body preserve/replace, content children under `+`/`~`, scenario
         rewrite via `~` on H3 under `@`, and pointer to `ds schema spec`

- [x] 3. Replace vague lightest-touch Quality with a short decision table; keep stable
         titles, cold-reader, no restatement-only rewrites

- [x] 4. Replace Example with one multi-marker block (`=`, `-`, `@` with scenario `-`/`+`,
         `+` requirement with unmarked `### Scenario`); fix invalid marked content
         children under `+`

- [x] 5. Parallel Markers/Rules/Quality updates in
         `crates/duckspec/content/schemas/doc-delta.md` (restate body/children rules; do
         not rely only on “see spec-delta”)

- [x] 6. Replace doc-delta Example with multi-marker (`-`, `~`, `+` sections); skim
         `ds schema spec-delta` / `doc-delta` for scannability
