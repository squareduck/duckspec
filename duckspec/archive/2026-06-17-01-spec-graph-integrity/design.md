# Spec-graph integrity across merge, scan, and archive — Design

Three focused additions in `duckpond`, each reusing the audit's existing resolver rather
than duplicating or bypassing it: a validated wrapper over the content-agnostic
`apply_delta`, a scan-boundary filter on the audit walk, and an orphan-projection check
the `ds archive` command runs before finalizing.

## Approach

Everything that mutates or scans the spec graph converges on one set of primitives the
audit already owns: `ScenarioKey`, `build_scenario_index`, `scan_source_files`, and the
resolution loop in `run_audit`. Today the merge path open-codes apply-then-parse three
different ways, the scan walks too wide, and archive never consults the resolver at all.
This change routes all three through the existing engine.

```text
              ┌──────────────────────────────────────────────────────┐
              │ audit resolver (existing)                            │
              │   ScenarioKey · build_scenario_index ·               │
              │   scan_source_files · resolution loop                │
              └───────▲───────────────▲───────────────▲──────────────┘
                      │ reuse          │ extend        │ reuse
        ┌─────────────┴──┐   ┌─────────┴────────┐   ┌──┴───────────────┐
        │ merge/validate │   │ audit/scan-      │   │ archive/         │
        │ (wrap apply_   │   │ boundary         │   │ backlink-guard   │
        │  delta + parse)│   │ (exclude +       │   │ (project post-   │
        │                │   │  nested skip)    │   │  archive index)  │
        └───────┬────────┘   └──────────────────┘   └────────┬─────────┘
                │ routed through                              │ called by
   ┌────────────┼───────────────┐                    ┌────────┴─────────┐
   │ status     audit    archive│                    │ ds archive run() │
   └────────────────────────────┘                    └──────────────────┘
```

Implementation order is `scan-boundary → backlink-guard → merge/validate`. The scan fix
lands first because it repairs a live `ds audit` (13 false positives) and because the
guard reuses the same corrected scan boundary.

## merge/validate — validated merge wrappers

`merge::apply_delta` already merges on the generic heading tree and is content-agnostic;
its doc-comment tells callers to re-parse the result with the right parser. That re-parse
is the part every caller open-codes differently (and two callers silently swallow). The
fix folds it into two thin wrappers that share one result type and one error type,
differing only in which parser validates.

```rust
// duckpond::merge

/// A validated merge outcome. `A` is the parsed artifact (`Spec` or `Document`).
pub enum Merged<A> {
    /// The delta updated the artifact; `rendered` is the new markdown and
    /// `artifact` is the re-parsed, schema-valid result.
    Updated { rendered: String, artifact: A },
    /// The delta deleted the whole artifact (`-` on H1).
    Deleted,
}

/// A merge that either failed to apply or produced output that did not re-parse.
#[derive(Debug, thiserror::Error)]
pub enum MergeValidateError {
    #[error("merge failed: {}", summarize_errors(.0))]
    Merge(Vec<MergeError>),
    #[error("merged result did not parse: {}", summarize_errors(.0))]
    Parse(Vec<ParseError>),
}

/// Render a multi-error failure as one readable line: the first message plus an
/// "(and N more)" count.
pub fn summarize_errors<E: std::fmt::Display>(errors: &[E]) -> String { todo!() }

pub fn merge_spec_delta(source: &str, delta: &str)
    -> Result<Merged<Spec>, MergeValidateError> { todo!() }

pub fn merge_doc_delta(source: &str, delta: &str)
    -> Result<Merged<Document>, MergeValidateError> { todo!() }
```

Each wrapper is ~4 lines: call `apply_delta`, map `Err → Merge`, on `Ok(None)` return
`Deleted`, on `Ok(Some(rendered))` parse with `parse_spec` / `parse_document`, map
`Err → Parse`, return `Updated`. `apply_delta` stays public and unchanged — the wrappers
layer on top.

### Caller routing

The three open-coded sites route through the wrappers. Failures stop being silent; how
they surface depends on the command's job:

```text
caller                        was                       becomes
────────────────────────────  ────────────────────────  ────────────────────────
archive::execute_plan         apply_delta + write,       merge_spec_delta /
  (spec.md / doc.md)          validate later, generic    merge_doc_delta by target
                              "; "-joined message        suffix; abort on Err
status::delta_new_coverage    apply_delta, swallow all,  merge_spec_delta; reuse
  (spec.md)                   re-parse with parse_spec    Updated.artifact; on Err
                                                          push a visible error line,
                                                          continue
audit::build_change_scenarios apply_delta, swallow all,  merge_spec_delta; reuse
  (spec.md)                   re-parse with parse_spec    Updated.artifact; on Err
                                                          record a counted report
                                                          entry
```

`status` and `audit` currently re-parse the merged string with `parse_spec` right after
merging — `Merged::Updated.artifact` hands them the already-parsed `Spec`, removing the
second parse entirely.

`audit` gains a new counted report category so a malformed change delta is no longer
invisible:

```rust
// duckpond::audit
pub struct ChangeMergeError {
    pub change_name: String,
    pub target: PathBuf,         // e.g. caps/auth/spec.md
    pub error: MergeValidateError,
}

pub struct AuditReport {
    // ...existing fields...
    pub change_merge_errors: Vec<ChangeMergeError>,   // NEW; folded into total_errors()
}
```

## audit/scan-boundary — exclude config + nested-project skip

The `@spec` scan walks too wide. `scan_source_files` builds an `ignore` `WalkBuilder` per
root and only filters out paths under the duckspec root itself, so it descends into nested
duckspec fixtures and reads illustrative markers in doc code examples — the 13 current
false positives. Two additions close the boundary.

First, `Config` learns an `exclude` list, parsed exactly like `test_paths`:

```rust
// duckpond::config
pub struct Config {
    pub test_paths: Vec<PathBuf>,
    pub exclude: Vec<PathBuf>,      // NEW — files/dirs omitted from backlink scanning
    pub format: FormatConfig,
}

pub enum ConfigError {
    // ...existing...
    #[error("config.toml: exclude must be an array of strings")]
    BadExclude,                     // NEW
}
```

Second, `scan_source_files` gains a `filter_entry` closure that prunes whole subtrees
before they are read — both nested projects and excluded paths:

```rust
// duckpond::audit
fn scan_source_files(project_root: &Path, duckspec_root: &Path, config: &Config)
    -> Result<Vec<SourceBacklink>, AuditError>
{
    let excluded: Vec<PathBuf> = config.exclude.iter()
        .map(|p| project_root.join(p))
        .filter_map(|p| p.canonicalize().ok())
        .collect();

    // for each scan root:
    let walker = WalkBuilder::new(root)
        .filter_entry(move |entry| {
            let path = entry.path();
            // prune nested duckspec projects (self-governing: own duckspec/caps/)
            if path.is_dir() && path.join("duckspec").join("caps").is_dir() {
                return false;
            }
            // prune explicitly excluded files/dirs
            if let Ok(c) = path.canonicalize() {
                if excluded.iter().any(|e| c.starts_with(e)) { return false; }
            }
            true
        })
        .build();
    // ...existing per-file read + scan_file, keep the duckspec-root skip...
}
```

`filter_entry` pruning a directory skips its whole subtree, which is what we want for both
a nested project and an excluded dir; for an excluded *file* the same predicate rejects
the single entry. Keying nested detection on `duckspec/caps/` (not a bare `duckspec/`)
avoids colliding with the `crates/duckspec/` source crate. The existing
`starts_with(duckspec_canonical)` content skip stays as the guard for the root project's
own tree.

## archive/backlink-guard — orphan projection

Archiving rewrites cap specs; a removed or renamed scenario can orphan a live `@spec`
backlink. The guard answers "would this archive leave any backlink unresolved that
resolves today?" by running the existing resolver against a *projected* post-archive index
— no new detection logic.

```rust
// duckpond::audit (or a small sibling module)

/// A capability's projected post-archive spec content.
pub enum ProjectedSpec {
    Updated(String),   // spec.md will have this content (copy or merged delta)
    Deleted,           // capability removed (delta `-` on H1)
}

/// Backlinks that resolve against the current caps but would not after the
/// projected archive lands. `projected` is keyed by cap path (e.g. "auth").
pub fn would_be_orphaned(
    project_root: &Path,
    duckspec_root: &Path,
    config: &Config,
    projected: &HashMap<String, ProjectedSpec>,
) -> Result<Vec<UnresolvedBacklink>, AuditError> { todo!() }
```

It reuses `build_scenario_index` for the current index, applies the projection (drop each
projected cap's old keys, add the keys parsed from its new content, or drop entirely on
`Deleted`), reuses `scan_source_files` (so it inherits the corrected scan boundary), and
reports `unresolved_after ∖ unresolved_before` — the set difference isolates orphans the
archive *causes* from any pre-existing breakage. Only `spec.md` projections matter; doc
deltas don't affect backlink resolution.

The `ds archive` command builds `projected` from the data it already computes —
`ArchiveResult.writes` holds the final merged spec content per cap — and calls the guard
between `execute_plan` and the irreversible `apply_results`/`rename`:

```text
build_plan → execute_plan ──→ would_be_orphaned(projected) ──→ apply_results → rename
                              │                              │
                              └ orphans & !--allow-orphans ──┴─→ bail, list files,
                                                                  nothing written
```

Refusal is the default; a new `--allow-orphans` flag on `ds archive` downgrades it to a
printed warning and proceeds. Because the guard runs before any write, a refusal leaves
the working tree untouched.

## Decisions

- **Generic `Merged<A>` + single `MergeValidateError`** — one result enum parameterized by
  artifact type and one error enum, rather than the salvage doc's separate
  `SpecMerge`/`DocMerge` + `SpecMergeError`/`DocMergeError` families. Alternatives: four
  bespoke types (rejected: near-duplicate, more to maintain); a `kind: ArtifactKind`
  parameter on one function (rejected: forces a union return that callers must re-match,
  and spec callers want the typed `Spec` directly).

- **Wrappers return the parsed artifact** — `Updated { artifact }` hands `status` and
  `audit` the parsed `Spec` they currently produce with a second `parse_spec` call.
  Alternative: validate-and-discard returning only the string (rejected: forces every spec
  caller to parse twice).

- **Guard reports the set difference** `unresolved_after ∖ unresolved_before` — blames the
  archive only for orphans it actually causes. Alternative: report all post-archive
  unresolved (rejected: would surface pre-existing breakage as if the archive caused it,
  and noisy until scan-boundary lands).

- **Guard runs pre-write on the plan's own merged content** — fed the same
  `ArchiveResult.writes` the archive will persist, so the projection cannot drift from
  what actually lands. Alternative: re-merge inside the guard (rejected: duplicate work
  and a divergence risk).

- **`ds status` reports and continues; `ds archive` refuses with `--allow-orphans`** — a
  malformed delta in `status` prints a visible error line and keeps rendering the
  dashboard; the archive guard hard-refuses by default with an escape hatch. Alternatives:
  status hard-fail (rejected: status is a read-only dashboard, one bad delta shouldn't
  blank it); guard warn-only (rejected: silent drift is the problem being fixed).

## Risks

- **`ignore`'s `filter_entry` is not invoked on the walk root** → the root project (which
  owns `duckspec/caps/`) must not be pruned; it is the scan root, not an entry, and is
  additionally covered by the existing `starts_with(duckspec_canonical)` skip. Confirm
  with a test that scanning a project containing a nested fixture keeps the parent's real
  backlinks and drops the nested ones.

- **Projection drift from actual writes** → the guard consumes the exact
  `ArchiveResult.writes` content, never a re-merge, so the projected index and the
  persisted files are built from the same bytes.

- **Surfacing makes `ds status` noisy on a broken change** → status emits one error line
  per failing delta and continues; it does not abort, so a single bad delta degrades
  gracefully rather than blanking the dashboard.

## Open questions

- None. Both prior open questions (status severity, guard severity) are resolved in
  Decisions.
