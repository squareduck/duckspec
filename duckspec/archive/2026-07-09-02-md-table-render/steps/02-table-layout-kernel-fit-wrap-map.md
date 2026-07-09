# Table layout kernel fit wrap map

Complete the pure kernel: fit columns to the pane with a minimum width, soft-wrap cells,
allow overflow total width when mins cannot fit, and bidirectional source ↔ fragment
mapping. Cover geometry and mapping scenarios with unit tests.

## Prerequisites

- [x] @step table-layout-kernel-recognition

## Tasks

- [x] 1. Implement fit-to-pane column widths (`MIN_COL_CHARS`, shrink longest-first) and
         soft cell wrap into fragments

- [x] 2. Compute per-row visual height and region `total_width_chars` (including overflow
         when mins exceed the pane)

- [x] 3. Implement `source_to_visual` and `visual_to_source` (or equivalent) for positions
         inside cells

- [x] 4. @spec editor/md-table Column fit and cell wrap: Short cells produce a total width within the pane

- [x] 5. @spec editor/md-table Column fit and cell wrap: A long cell soft-wraps within the pane

- [x] 6. @spec editor/md-table Column fit and cell wrap: Many minimum-width columns may exceed the pane

- [x] 7. @spec editor/md-table Source mapping: Fragment position maps into the cell’s source text

- [x] 8. @spec editor/md-table Source mapping: Source position in a cell maps to a fragment of that cell
