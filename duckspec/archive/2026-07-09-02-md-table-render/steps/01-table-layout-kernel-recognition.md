# Table layout kernel recognition

Add the pure `widget/md_table` module: detect complete GFM pipe tables, parse cells and
aligns, treat the separator as metadata, and emit cell-only display fragments. Cover
recognition and display-text scenarios with unit tests.

## Tasks

- [x] 1. Add `crates/duckboard/src/widget/md_table.rs` and register it in `widget.rs`

- [x] 2. Implement GFM table detection and cell/align parsing (`layout_tables` skeleton +
         region structure from the design)

- [x] 3. @spec editor/md-table Table recognition: Complete header, separator, and body form a region

- [x] 4. @spec editor/md-table Table recognition: Missing separator or body yields no region

- [x] 5. @spec editor/md-table Table recognition: Body column count mismatch yields no region

- [x] 6. @spec editor/md-table Separator, aligns, and display text: Separator is not a data row and defines aligns

- [x] 7. @spec editor/md-table Separator, aligns, and display text: Fragments omit pipe delimiters
