# Atomic session writes

Route session persistence through a temp-file + rename helper so an interrupted write
never truncates the previous file.

## Tasks

- [x] 1. Add a `write_atomic(path, data)` helper in `chat_store.rs` that writes to a temp
         file in the destination directory, then renames it into place; remove the temp
         file on write failure so no `.tmp` residue is left.

- [x] 2. Route `save_session` through `write_atomic` instead of `std::fs::write`.

- [x] 3. @spec chat/persistence Atomic session writes: A failed save leaves the prior contents intact
