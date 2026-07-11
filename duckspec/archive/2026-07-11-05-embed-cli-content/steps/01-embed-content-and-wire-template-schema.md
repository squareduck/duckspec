# Embed content and wire template/schema

Add `include_dir`, a `content` module over the stock tree, and switch `template` and
`schema` to read only from the embed.

## Tasks

- [x] 1. Add `include_dir` to `crates/duckspec/Cargo.toml`

- [x] 2. Add `crates/duckspec/src/content.rs` with
         `include_dir!("$CARGO_MANIFEST_DIR/content")` and helpers: `template`, `schema`,
         `command_files`, `has_harness` (wire the module from `main.rs` or `lib` as the
         crate structure requires)

- [x] 3. Switch `cmd/template.rs` to `content::template`; drop runtime
         `CARGO_MANIFEST_DIR` reads; update the unit test that walked `TEMPLATE_DIR` to
         use the embed

- [x] 4. Switch `cmd/schema.rs` to `content::schema`; drop runtime `CARGO_MANIFEST_DIR`
         reads

- [x] 5. @spec cli/stock-content Stock content from the binary: Known template is printed

- [x] 6. @spec cli/stock-content Stock content from the binary: Known schema is printed

- [x] 7. @spec cli/stock-content Clear unknown-name failures: Unknown template is rejected by name

- [x] 8. @spec cli/stock-content Clear unknown-name failures: Unknown schema is rejected by name
