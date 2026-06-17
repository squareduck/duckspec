# Spec-graph integrity across merge, scan, and archive

Make duckspec's integrity guarantees consistent across the paths that mutate or scan the
spec graph: fix a currently-broken `ds audit`, guard archive against orphaned `@spec`
backlinks, and turn silent merge failures into surfaced errors.

## Motivation

`ds audit` is broken today. Run at the repo root it reports 13 false-positive unresolved
backlinks and zero real ones, so the command is effectively unusable. The false positives
come from `@spec` markers the scan should never have looked at: nested duckspec test
fixtures (which resolve against their own specs) and illustrative markers inside
documentation code examples.

Underneath that, the integrity guarantees are inconsistent. The audit engine already
resolves every `@spec` backlink, but the paths that mutate or finalize the spec graph
don't reuse that guarantee:

- Delta-merge failures are silently swallowed in `ds status` and `ds audit` — a malformed
  delta produces empty coverage data with no error.

- `ds archive` can finalize a capability while leaving live `@spec` backlinks pointing at
  scenarios that no longer exist. The next audit would flag them, but nothing stops the
  archive from creating the drift in the first place.

These are three faces of one problem: the audit already knows what a healthy spec graph
looks like; the scan boundary, the merge path, and the archive path need to honor that
consistently.

## Scope

```
caps/
├── parse/                      (existing — untouched)
├── merge/
│   └── validate/        ← NEW  validated delta-merge entry point
├── audit/
│   └── scan-boundary/   ← NEW  which files the @spec scan includes
└── archive/
    └── backlink-guard/  ← NEW  refuse to orphan live backlinks
```

These are the first non-`parse` capabilities in the project. Each is scoped to the
behavior this change owns, not a full backfill of its subsystem.

### New capabilities

- `merge/validate` — a single validated entry point for delta merges, one blessed path per
  artifact kind (spec, doc). Returns a typed merged-or-deleted result or a typed error
  distinguishing a merge failure from a post-merge parse failure, with a
  `summarize_errors` helper that renders many errors as one readable line. `ds status`,
  `ds audit`, and `ds archive` route through it, so merge and parse failures become
  surfaced errors instead of silent no-ops.

- `audit/scan-boundary` — the file boundary of the `@spec` backlink scan: existing
  `test_paths` scoping, a new `exclude` key in `config.toml` (`Vec<PathBuf>`, with a
  `ConfigError::BadExclude` for a non-array value), and automatic skipping of nested
  duckspec projects (any directory that owns its own `duckspec/caps/`), which never need
  listing in `exclude`.

- `archive/backlink-guard` — before finalizing an archive, project the post-archive
  scenario index, run the existing backlink resolver against it, and refuse (or loudly
  warn) when archiving a capability would orphan live `@spec` backlinks, naming the
  offending files.

### Out of scope

- The delta-merge algorithm itself (`apply_delta` is pre-existing; only the validating
  wrapper and consistent errors are new).

- The rest of the audit engine — artifact checks, step coverage, and other resolution
  logic are unchanged; only the scan boundary moves.

- GUI edge auto-scroll, which remains a separate standalone change.

- Any ducknest daemon, Telegram, or daemon-backed state work.

## Impact

```
   scan boundary            merge path                 archive path
   ─────────────            ──────────                 ────────────
   config.exclude           merge/validate             backlink-guard
   nested-skip   ┐          (one path)    ┐            (reuse resolver)
                 ▼                         ▼                  │
        ┌──────────────────── audit resolver ────────────────┘
        │  already resolves every @spec backlink (reused, not duplicated)
        └────────────────────────────────────────────────────────────────
```

- `duckpond::config` gains an `exclude` field and `ConfigError::BadExclude`.

- `duckpond::audit` scan gains nested-project skipping and `exclude` filtering; this fixes
  the 13 current false positives.

- `duckpond::merge` gains validated wrappers, typed merge/parse error types, and
  `summarize_errors`.

- `duckspec::cmd::{status, audit, archive}` change behavior: `status` and `audit` no
  longer silently swallow merge or parse failures, and `archive` gains the orphan guard.
  CLI output wording shifts accordingly.

- No external API or on-disk format breakage.

Implementation sequence: `3 → 2 → 1` — fix the live scan bug first, then the archive guard
that reuses the resolver, then the merge consolidation.
