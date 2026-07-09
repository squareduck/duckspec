# Shared soft-wrap helper

Collapse the duplicated space-preferring char-grid soft-wrap so prose wrap and table cell
fragments share one implementation.

## Prerequisites

- [x] @step table-layout-kernel-fit-wrap-map
- [x] @step hit-test-chrome-and-chat-enable

## Context

Addresses the soft-wrap duplication finding in `reviews/01-post-implementation-review.md`.

`md_table::soft_wrap_cell` (`crates/duckboard/src/widget/md_table.rs`) and
`wrap_line_starts` (`crates/duckboard/src/widget/text_edit/render.rs`) implement the same
algorithm with different output shapes (fragment ranges vs start offsets). Extract a
shared helper and thin both call sites.

Prefer a pure helper with no iced coupling so `md_table` can keep its unit-test boundary
(e.g. live next to `md_table` or as a small free function both modules can call). Do not
change wrap semantics; existing kernel and hybrid tests must stay green.

## Tasks

- [x] 1. Extract a shared soft-wrap primitive (char-grid, space-preferring breaks) used by
         both prose and table cells

- [x] 2. Rewire `wrap_line_starts` to build start offsets from the shared helper

- [x] 3. Rewire `soft_wrap_cell` / cell fragment build to use the same helper

- [x] 4. Run `md_table` and hybrid layout unit tests; confirm long-cell soft- wrap and
         prose wrap still behave as before
