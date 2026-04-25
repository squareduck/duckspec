# Tighten parse_marked_heading

Add the `MarkerMissingSpace` error variant and update `parse_marked_heading` so delta
headings reject markers that are not followed by an ASCII space.

## Tasks

- [x] 1. Add `MarkerMissingSpace { span: Span }` variant to `ParseError` in
         `crates/duckpond/src/error.rs`. Slot it immediately after `MissingDeltaMarker`
         inside the "Delta errors" block. Use display message
         `"delta marker must be followed by a space"` and label
         `"missing space after marker"`.

- [x] 2. Update `parse_marked_heading` in `crates/duckpond/src/parse/delta.rs` to require
         an ASCII space directly after the marker character. If the byte after the marker
         is anything other than `0x20`, push `ParseError::MarkerMissingSpace { span }` and
         return `None`. Multiple spaces remain accepted (the trailing text is `trim()`-ed;
         `ds format` normalizes to one space at write time).

- [x] 3. Add `MarkerMissingSpace { span }` to the `ParseError::span()` match arm in
         `crates/duckpond/src/error.rs`. Run `cargo test -p duckpond` to confirm existing
         fixtures (which all use the canonical form) still pass.
