# Hybrid layout Arc cache

Share hybrid `EditorLayout` through `Arc` in the text-edit hybrid cache so cache hits
clone a refcount instead of deep-copying table regions, cells, and fragments.

## Tasks

- [x] 1. Change `InternalState.hybrid_layout` in
         `crates/duckboard/src/widget/text_edit/render.rs` to store `Arc<EditorLayout>`
         and make `cached_hybrid_layout` return `Arc<EditorLayout>` (clone Arc on hit;
         `Arc::new` on miss)

- [x] 2. Update `TextEdit` layout, update, and draw call sites to use the shared `Arc`
         (via `as_ref()` or local binding) without deep-cloning the layout tree on the hit
         path

- [x] 3. Adjust existing hybrid-layout unit tests (including
         `cached_hybrid_layout_reuses_until_key_changes`) for the Arc API

- [x] 4. @spec chat/stream-ui Hybrid layout reuse: Second hybrid layout request with the same key does not recompute tables

- [x] 5. @spec chat/stream-ui Hybrid layout reuse: Cache hit shares layout geometry without deep-cloning the tree
