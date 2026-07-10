# Live editor in-place refresh

When materializing, keep settled editors and refresh a suffix-growing live answer or
thinking block in place instead of `EditorState::new` + full-buffer highlight every time.

## Prerequisites

- [x] @step materialize-gate-and-stream-tick

## Tasks

- [x] 1. In `rebuild_chat_editor` (`crates/duckboard/src/area/interaction.rs`), detect
         suffix-only growth of a live answer/thinking block (shared line prefix, optional
         last-line extend, then new lines) versus full reshape

- [x] 2. Implement in-place refresh (`Arc::make_mut` line updates, partial re-highlight of
         the dirty line range, `highlight_version` bump) and keep the existing full
         `EditorState::new` + full highlight path for reshape / new indices

- [x] 3. @spec chat/stream-ui Settled and live editor refresh: Unchanged settled block keeps its editor across materialize

- [x] 4. @spec chat/stream-ui Settled and live editor refresh: Suffix-growing live answer refreshes in place

- [x] 5. @spec chat/stream-ui Settled and live editor refresh: Block list reshape uses full rebuild for affected indices
