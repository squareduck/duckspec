# Pin parse delta

Wire `@spec` backlinks for the 9 `parse/delta` scenarios. 3 are backed by existing
happy-path fixtures; 6 require new error fixtures (5 delta-specific errors plus the new
`marker_missing_space.md` exercising the Step 01 tightening).

## Prerequisites

- [ ] @step tighten-parse-marked-heading

## Context

Create new fixture files under `crates/duckpond/tests/fixtures/delta/errors/` and matching
`expect_err`+`assert_debug_snapshot` tests in `crates/duckpond/tests/parse_delta.rs`. Add
`// @spec parse/delta <Requirement>: <Scenario>` comments above each test.

The 6 new fixtures:

```
tests/fixtures/delta/errors/
├── add_on_h1.md                       ← AddOnH1 (`# + Foo`)
├── anchor_on_h3.md                    ← AnchorOnH3 (`### @ Foo` under @ parent)
├── marker_missing_space.md            ← MarkerMissingSpace (`# @Foo` no space)
├── marker_on_content_child.md         ← MarkerOnContentChild (marker H3 under ~/+)
├── missing_marker_on_h2.md            ← MissingDeltaMarker (`## Plain text H2`)
└── remove_and_rename_invariants.md    ← NonEmptyRemoveBody + InvalidRenameEntry
```

The combined `remove_and_rename_invariants.md` fixture should contain both a `-` entry
with a non-empty body and a `=` entry with an empty body so a single snapshot captures
both error variants.

## Tasks

- [x] 1. @spec parse/delta Delta document structure: Delta with a single Add entry parses successfully

- [x] 2. @spec parse/delta Delta document structure: Delta with mixed-marker entries is sorted into canonical order

- [x] 3. @spec parse/delta Delta document structure: Delta with rename and anchor entries parses with new-name extraction

- [x] 4. @spec parse/delta Marker rules: Marker without a following space raises MarkerMissingSpace

- [x] 5. @spec parse/delta Marker rules: Add marker on H1 raises AddOnH1

- [x] 6. @spec parse/delta Marker rules: H2 without a marker raises MissingDeltaMarker

- [x] 7. @spec parse/delta Marker rules: Anchor marker on H3 raises AnchorOnH3

- [x] 8. @spec parse/delta Marker rules: Marker on a content child raises MarkerOnContentChild

- [x] 9. @spec parse/delta Per-marker entry semantics: Remove with body or rename without new-name line raises the relevant invariant
