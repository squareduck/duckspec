# AGENTS.md

## Version Control

This project uses **jujutsu (jj)** instead of git.

- Use `jj` commands for all version control operations
- Do NOT use `git` commands
- Common operations:
  - `jj status` — show working copy status
  - `jj diff` — show changes
  - `jj log` — show commit history
  - `jj new` — create a new change on top of current
  - `jj commit -m "message"` — commit current changes
  - `jj describe -m "message"` — update current change description
  - `jj bookmark set <name>` — set a bookmark (similar to git branch)

## Commit Rules

- **NEVER commit automatically** — always show the suggested commit message and wait for explicit user confirmation before running `jj commit`
- Do NOT run destructive jj commands (like `jj abandon`, `jj squash --force`) without explicit confirmation

## Commit Message Format

```
type(optional-change): short description

- optional summarized changes
```

**Types:** `feat`, `fix`, `chore`, `doc`, `refactor`

Examples:
- `chore: initial project scaffold`
- `feat: implement validate command`
- `doc: write specs for validation area`
- `refactor: simplify merge algorithm`

## Conventions

### Workspace structure

Nested Cargo workspace under `crates/`:

- `crates/duckpond/` — core library (crate name: `duckpond`)
- `crates/duckspec/` — CLI crate (crate name: `duckspec`, binary name: `ds`)
- `crates/duckboard/` — GUI crate (crate name: `duckboard`, binary name: `duckboard`)

Directory names match crate names.

Research notes are under `references/`:

- `references/duckspec.md` — core duckspec workflow design

### Error handling

- `duckpond` (library) uses `thiserror` for typed error enums
- `duckspec` and `duckboard` (binaries) use `anyhow` for application-level error wrapping
- Do not use `anyhow` inside `duckpond`

### Module style

- Use the `foo.rs` + `foo/` subdirectory pattern for modules with children
- Avoid `mod.rs` files
- Exception: integration test helper modules under `tests/common/mod.rs` are
  allowed. Cargo treats any file directly under `tests/` as its own test
  binary, so `tests/common/mod.rs` is the only friction-free way to share
  helpers between integration test binaries without `#[path]` attributes.

### Testing

This project uses three layers of tests and a standard snapshot-testing setup.

#### Test layers

- **Unit tests** live inline in `src/` files under `#[cfg(test)] mod tests { … }`.
  Use them for small, pure functions and internal helpers that don't need
  fixtures.
- **Integration tests** live in each crate's `tests/` directory and exercise
  the crate from the outside, as an external consumer would. Parser tests,
  end-to-end behavior tests, and fixture-driven tests belong here.
- **Doc tests** in `///` comments are encouraged for public API examples but
  are not a substitute for unit or integration tests.
