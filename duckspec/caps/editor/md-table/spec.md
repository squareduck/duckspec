# Markdown table layout

Pure GFM pipe-table layout over a line buffer: recognize complete tables, fit columns to a
character-cell pane with soft cell wrap, expose geometry without pipe glyphs or a
separator data row, and map visual positions back to source positions.

## Requirement: Table recognition

A layout pass SHALL emit a table region only for a complete GFM pipe table: a header pipe
row, a following separator row that defines column alignments, and one or more body pipe
rows with the same column count as the header. Lines that do not form such a block SHALL
produce no region.

> test: code

### Scenario: Complete header, separator, and body form a region

- **GIVEN** consecutive lines that are a header pipe row, a valid separator row, and at
  least one body pipe row with matching column count

- **WHEN** the lines are laid out as tables

- **THEN** the result contains exactly one table region covering those lines

> test: code
> - crates/duckboard/src/widget/md_table.rs:625

### Scenario: Missing separator or body yields no region

- **GIVEN** a header pipe row without a following valid separator and body, or a header
  and separator without a body row

- **WHEN** the lines are laid out as tables

- **THEN** the result contains no table region

> test: code
> - crates/duckboard/src/widget/md_table.rs:650

### Scenario: Body column count mismatch yields no region

- **GIVEN** a header, a valid separator, and a body row whose column count differs from
  the header

- **WHEN** the lines are laid out as tables

- **THEN** the result contains no table region

> test: code
> - crates/duckboard/src/widget/md_table.rs:666

## Requirement: Column fit and cell wrap

Given a pane width in character cells, the layout SHALL size columns so the table’s total
width fits within that pane when possible without shrinking any column below the minimum
character width. Cell text SHALL soft-wrap to its column width; a logical row’s visual
height SHALL be the maximum fragment count across its cells. When every column is already
at the minimum width and the table still cannot fit, the region’s total width SHALL exceed
the pane width.

> test: code

### Scenario: Short cells produce a total width within the pane

- **GIVEN** a complete table whose natural column widths sum to less than the pane width
- **WHEN** the table is laid out for that pane
- **THEN** the region’s total width is less than or equal to the pane width

> test: code
> - crates/duckboard/src/widget/md_table.rs:747

### Scenario: A long cell soft-wraps within the pane

- **GIVEN** a complete table with a cell whose text is longer than its fitted column width
- **AND** the table can still fit the pane at or above minimum column widths
- **WHEN** the table is laid out for that pane
- **THEN** that cell has more than one display fragment
- **AND** the logical row’s visual height is greater than one
- **AND** the region’s total width is less than or equal to the pane width

> test: code
> - crates/duckboard/src/widget/md_table.rs:764

### Scenario: Many minimum-width columns may exceed the pane

- **GIVEN** a complete table with enough columns that the sum of minimum column widths
  exceeds the pane width

- **WHEN** the table is laid out for that pane

- **THEN** the region’s total width is greater than the pane width

> test: code
> - crates/duckboard/src/widget/md_table.rs:797

## Requirement: Separator, aligns, and display text

The separator row SHALL contribute column alignments only and SHALL NOT appear as a data
row. Display fragments SHALL cover cell text only — not `|` delimiters.

> test: code

### Scenario: Separator is not a data row and defines aligns

- **GIVEN** a complete table whose separator uses left, center, and right align markers
- **WHEN** the table is laid out
- **THEN** the region has no data row for the separator line
- **AND** the region’s column alignments match the separator markers

> test: code
> - crates/duckboard/src/widget/md_table.rs:673

### Scenario: Fragments omit pipe delimiters

- **GIVEN** a complete table whose source lines include `|` cell delimiters
- **WHEN** the table is laid out
- **THEN** no display fragment’s text is a `|` delimiter
- **AND** each fragment’s text is a contiguous slice of some cell’s source text

> test: code
> - crates/duckboard/src/widget/md_table.rs:692

## Requirement: Source mapping

A position inside a cell display fragment SHALL map to a source position on that row’s
line within the cell’s source text. A source position within a cell SHALL map to a display
fragment of that cell.

> test: code

### Scenario: Fragment position maps into the cell’s source text

- **GIVEN** a laid-out table with at least one non-empty cell fragment
- **WHEN** a position inside that fragment is mapped to source
- **THEN** the source position lies on the fragment’s row line
- **AND** the source position lies within that cell’s source text range

> test: code
> - crates/duckboard/src/widget/md_table.rs:819

### Scenario: Source position in a cell maps to a fragment of that cell

- **GIVEN** a laid-out table with a cell whose source text is non-empty
- **WHEN** a source position inside that cell’s text is mapped to the visual layout
- **THEN** the result identifies a display fragment of that same cell

> test: code
> - crates/duckboard/src/widget/md_table.rs:852
