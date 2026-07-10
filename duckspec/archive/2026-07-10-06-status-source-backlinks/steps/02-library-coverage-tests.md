# Library coverage tests

Cover linkage and snapshot membership for `for_change` with tempdir fixtures (same style
as `crates/duckpond/tests/audit.rs`). Write backlink-bearing fixture source with inline
`\n` escapes so this project's own audit does not treat fixtures as live backlinks.

## Prerequisites

- [x] @step change-coverage-helper

## Tasks

- [x] 1. Add `crates/duckpond/tests/change_coverage.rs` (or extend an existing integration
         binary) with helpers to write change caps, deltas, and optional source files

- [x] 2. @spec status/change-coverage Source backlink is the linkage signal: Resolving source backlink makes the scenario linked

- [x] 3. @spec status/change-coverage Source backlink is the linkage signal: Marker path list without a source backlink leaves the scenario open

- [x] 4. @spec status/change-coverage Source backlink is the linkage signal: A linked scenario is not reported as open

- [x] 5. @spec status/change-coverage Snapshot is change-introduced test code only: New change-cap test:code scenario is included

- [x] 6. @spec status/change-coverage Snapshot is change-introduced test code only: Delta-introduced test:code scenario is included

- [x] 7. @spec status/change-coverage Snapshot is change-introduced test code only: Pre-existing base scenario is excluded

- [x] 8. @spec status/change-coverage Snapshot is change-introduced test code only: Non-test:code scenario is excluded

- [x] 9. @spec status/change-coverage Change status surfaces the partition: Step checkbox state does not change linkage
