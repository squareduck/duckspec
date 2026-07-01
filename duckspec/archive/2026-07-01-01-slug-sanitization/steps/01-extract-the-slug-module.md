# Extract the slug module

Create the shared `duckpond::slug` module holding the canonical slug rule, and repoint the
step parser at it — deleting the duplicate in `artifact::step`.

## Tasks

- [x] 1. Create `crates/duckpond/src/slug.rs` with `pub fn slugify(title: &str) -> String`
         (body lifted verbatim from `artifact::step::slugify`), and declare
         `pub mod slug;` in `crates/duckpond/src/lib.rs`.

- [x] 2. Repoint `parse/step.rs:31` to call `crate::slug::slugify`, then delete
         `pub fn slugify` at `artifact/step.rs:66` and its unit tests. Confirm
         `cargo test -p duckpond` still passes the existing `parse_step` tests unchanged.

- [x] 3. @spec slug Slug transformation: Words become lowercase, dash-joined tokens

- [x] 4. @spec slug Slug transformation: A run of non-alphanumeric characters collapses to one dash

- [x] 5. @spec slug Slug transformation: Leading and trailing non-alphanumeric characters are dropped

- [x] 6. @spec slug Slug transformation: Unicode alphanumerics are preserved

- [x] 7. @spec slug Slug transformation: A title with no alphanumeric characters yields an empty string
