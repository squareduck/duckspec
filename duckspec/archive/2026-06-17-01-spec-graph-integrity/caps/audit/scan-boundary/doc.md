# Backlink scan boundary

The set of rules that decide which source files the audit reads when it looks for `@spec`
backlinks. A backlink only counts if it lives inside the boundary; files outside it are
never read and never produce backlinks.

The boundary exists because not every `@spec`-looking marker in a tree is a real backlink.
Documentation can show example markers, and a repository can contain other, self-contained
duckspec projects (test fixtures, vendored samples) whose markers belong to their own
specs. Reading those markers against the enclosing project's specs produces false
"unresolved backlink" reports. The boundary keeps the scan to genuine backlinks.

## Scan roots

The roots are where the walk starts.

```text
test_paths set?
   ├── yes → scan each existing test_paths entry (relative to project root)
   └── no  → scan the whole project root
```

A `test_paths` entry that does not exist on disk is skipped silently, so a config listing
several candidate test directories does not break when only some are present.

## Exclusions

Three rules remove paths from the scan. They compose: a path is scanned only if it
survives all three.

```text
| Rule                  | Removes                                  | Configured by      |
| --------------------- | ---------------------------------------- | ------------------ |
| duckspec self         | the project's own duckspec/ tree         | automatic          |
| nested project        | any dir owning its own duckspec/caps/    | automatic          |
| exclude list          | named files and directory subtrees       | config.toml exclude|
```

**duckspec self.** The project's own `duckspec/` directory is never scanned for backlinks
— the specs and codex live there, not source code.

**Nested projects.** A directory that owns its own `duckspec/caps/` is a self-governing
project: its `@spec` markers resolve against *its* specs, so the enclosing scan skips it
and its whole subtree. Detection keys on `duckspec/caps/` specifically, not a bare
`duckspec/` directory, so a source directory merely named `duckspec` (such as a crate
directory) is not mistaken for a project. Nested projects never need to be listed in
`exclude`.

**Exclude list.** The `exclude` key in `config.toml` is an array of paths, relative to the
project root, that the scan omits. Naming a file omits that file; naming a directory omits
the directory and everything beneath it. This is the escape hatch for individual files
that contain example or illustrative markers — design docs, reference material, single
test files — that are not nested projects.

```toml
exclude = ["references/design.md", "crates/foo/tests/fixture.rs"]
```

When `exclude` is absent it defaults to empty. When it is present but not an array of
strings, configuration loading fails with a `BadExclude` error rather than silently
ignoring the malformed value.

## Relationship to backlink resolution

The boundary decides *which* backlinks exist; it does not decide whether they resolve.
Every backlink the scan returns is then matched against known scenarios, and any that fail
to resolve are reported. By keeping the boundary tight, the resolution step sees only
backlinks that are genuinely expected to resolve against this project's specs.
