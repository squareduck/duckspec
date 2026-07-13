# Changed files row cache

Precompute owned flat rows for the Change “Changed Files” section so `view()` maps the
cache instead of rebuilding and flattening a `FileTree` every frame.

## Prerequisites

- [x] @step stream-tick-need

## Tasks

- [x] 1. Add `ChangedFileRow` (owned) and `changed_file_rows` on `change::State` in
         `crates/duckboard/src/area/change.rs`; implement `rebuild_changed_file_rows` from
         today’s tree insert + flatten

- [x] 2. Rebuild the cache at the end of `set_changed_files` and on
         `Message::ToggleFileDir` (after expand set mutates)

- [x] 3. Change `view_changed_files_section` to map `state.changed_file_rows` → `ListRow`
         only (no per-view tree build)

- [x] 4. Add unit tests that set/toggle updates row cache content; run focused change-area
         tests
