# Embed CLI content - Design

Compile-time embed of `crates/duckspec/content/` into the `ds` binary; `init`, `template`,
and `schema` read only from that embed at runtime.

## Approach

Today three commands resolve stock files via a baked absolute path:

```
env!("CARGO_MANIFEST_DIR") + "/content/…"  ──fs::read_*──►  disk at build path
```

Target shape: one embed root, three thin command adapters:

```
content/
  commands/{claude,opencode}/*.md
  templates/*.md
  schemas/*.md
         │
         │  include_dir! (compile time)
         ▼
  crates/duckspec/src/content.rs   ── static Dir + lookup helpers
         │
    ┌────┼────────────┐
    ▼    ▼            ▼
  init  template    schema
  (write) (print)   (print)
```

No runtime `CARGO_MANIFEST_DIR` for stock content. Project hooks under `duckspec/hooks/`
stay filesystem-based (unchanged).

## Embedded content module

New module `crates/duckspec/src/content.rs` owns the tree and lookup API:

```rust
use include_dir::{Dir, include_dir};

static CONTENT: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/content");

/// UTF-8 body of `templates/{name}.md`, or `None` if missing.
pub fn template(name: &str) -> Option<&'static str> { /* … */ }

/// UTF-8 body of `schemas/{name}.md`, or `None` if missing.
pub fn schema(name: &str) -> Option<&'static str> { /* … */ }

/// Files under `commands/{harness}/` whose names end in `.md`.
/// Each entry is `(file_name, utf8_body)`.
pub fn command_files(harness: &str) -> impl Iterator<Item = (&'static str, &'static str)> { /* … */ }

/// Whether `commands/{harness}/` exists in the embed (known harness tree).
pub fn has_harness(harness: &str) -> bool { /* … */ }
```

- Paths inside the embed use `/` (include_dir convention).

- Bodies are UTF-8 markdown; invalid UTF-8 is a programming error (`.expect` or
  pre-validated at load helpers) — stock content is always UTF-8 today.

- Unknown names return `None` / empty iterator; callers map to `anyhow!("unknown …")` —
  never raw I/O errors for missing stock content.

## Command adapters

**`cmd/init.rs`**

- Keep `HARNESS_COMMAND_DIR` for target install paths (`.claude/commands`,
  `.opencode/commands`).

- Drop `COMMANDS_DIR` and `fs::read_dir` / `fs::copy` of source files.

- For a known harness: `create_dir_all` target, then for each
  `content::command_files(harness)` write body with `fs::write` and print
  `installed <dest>` as today.

- Unknown harness: same error text as today (supported list from the static table, not
  from embed discovery — keeps the CLI contract explicit).

**`cmd/template.rs`**

- Load body via `content::template(&name)` instead of `fs::read_to_string`.

- Hook injection (`duckspec/hooks/{stage}-before|after.md`) unchanged.

- Unit test that every stock template has `## Before write` / `## After write`: iterate
  embed (or a small helper list) instead of `read_dir(TEMPLATE_DIR)`.

**`cmd/schema.rs`**

- Load via `content::schema(&name)`; print on success; `unknown schema: {name}` on miss.

## Dependency

- Add `include_dir` to `crates/duckspec` only (not duckpond/duckboard).

- Macro expands at compile time from `$CARGO_MANIFEST_DIR/content` — CI and local builds
  still need the tree **at build time** (already true of the crate layout). Runtime needs
  only the binary.

## Impact

- **`crates/duckspec` dependency:** `include_dir`

- **Binary size:** ~48 small markdown files; negligible

- **Release tarball:** still ships only `ds`; now self-sufficient for init/template/schema

- **`cargo install --path` / CI `cargo build`:** unchanged source layout; rebuild after
  editing `content/` picks up changes via include_dir’s dependency tracking

- **No duckpond / duckboard / public library API change**

- **Tests:** unit tests that walked `TEMPLATE_DIR` on disk move to the embed; integration
  coverage for init can assert install works without a fake source tree (optional but
  valuable)

## Decisions

- **`include_dir` over `rust-embed` / hand-rolled `include_str!` map** — directory tree +
  path lookup matches the existing layout; no derive boilerplate; no 48-line manual map.
  Alternative: `rust-embed` (heavier feature set we do not need). Alternative: build.rs
  codegen (more moving parts).

- **Single embed root + helpers, not three separate `include_dir!`s** — one static, one
  place that knows path conventions.

- **Harness allow-list stays in `init`** — embed holds files; CLI still owns which harness
  names are public. Avoids “empty directory in embed” becoming an accidental public
  harness.

- **Write with `fs::write`, not copy** — source is `&str` in the binary, not a path.

## Risks

- **UTF-8 assumptions** → stock files are author-controlled markdown; fail loud at first
  access if ever broken rather than silent lossy conversion.

- **Stale mental model (“edit content/ and re-run binary without rebuild”)** → same as any
  embedded asset; only matters for local contrib workflow. Document in README only if we
  touch install docs; not a product behavior change.

- **include_dir path separator / `$CARGO_MANIFEST_DIR`** → use the crate’s documented
  `$CARGO_MANIFEST_DIR/content` form so Windows/Linux/macOS builds stay consistent.
