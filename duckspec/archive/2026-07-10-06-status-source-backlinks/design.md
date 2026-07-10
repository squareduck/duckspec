# Change status uses source backlinks — Design

Teach `ds status <change>` to measure change `test:code` progress against resolving source
`@spec` backlinks via a small duckpond helper — never via marker path lists, and never by
calling `run_audit`.

## Approach

```text
ds status <change>
        │
        ▼
┌───────────────────────────────────────────────────────────┐
│ duckpond::change_coverage::for_change(...)                │
│                                                           │
│  1. project change-introduced scenarios                   │
│     (ChangeCapSpec all + SpecDelta new-only)              │
│  2. keep test:code only                                   │
│  3. scan source @spec (Config roots / excludes)           │
│  4. linked = key ∈ backlink set                           │
│     open   = key ∉ backlink set                           │
└───────────────────────────────────────────────────────────┘
        │
        ▼
  test coverage:  N/M linked
    open:          @spec …   (only unlinked; never linked)

  steps / reviews / proposal chrome unchanged
```

**Status stays a dashboard.** The helper returns facts; the CLI prints progress. No exit
code, no step-claim classification, no artifact schema walk. Audit continues to own the
integrity gate (`audit/change-progress` untouched).

**Ground truth for “linked”** is a resolving source `@spec` key — the same notion audit
uses when deciding whether a scenario has a backlink. Marker `> - path:line` lists are
ignored for this tally.

Global `ds status` (project overview) is unchanged.

## Change coverage helper

New duckpond module: `crates/duckpond/src/change_coverage.rs`, exported from `lib.rs`.
Owns the progress snapshot only.

```rust
// crates/duckpond/src/change_coverage.rs

/// Progress snapshot for a single change's test:code scenarios.
pub struct ChangeCoverage {
    /// Change-introduced test:code scenarios with at least one resolving source @spec.
    pub linked: Vec<ScenarioKey>,
    /// Change-introduced test:code scenarios with no resolving source @spec.
    pub open: Vec<ScenarioKey>,
    /// Spec deltas that failed to merge/re-parse while projecting scenarios.
    /// Status prints these and continues (same spirit as today's status merge errors).
    pub merge_errors: Vec<ChangeMergeError>,
}

/// Project change-introduced test:code scenarios and partition by source backlink
/// resolution. Does not validate artifacts, steps, or exit semantics.
pub fn for_change(
    duckspec_root: &Path,
    project_root: &Path,
    config: &Config,
    change_name: &str,
) -> Result<ChangeCoverage, ChangeCoverageError> {
    todo!()
}

pub enum ChangeCoverageError {
    Io { path: PathBuf, source: std::io::Error },
    // … other hard failures that prevent any snapshot
}
```

`ScenarioKey` and `ChangeMergeError` stay the shared types already defined in
`duckpond::audit` (or re-exported) so status and audit speak the same key language.

Partition rule:

```text
for each change-introduced test:code key k:
  if k ∈ source_backlink_keys  →  linked
  else                         →  open
```

A scenario with a correct source backlink is never in `open`. Marker path lists are not
consulted.

## Scenario projection and source scan (shared guts)

Today projection and scanning live as private helpers inside `audit.rs`:

- `build_change_scenarios` — new caps + delta-introduced scenarios
- `scan_source_files` — `Config` test_paths / exclude, nested-duckspec prune
- `backlink_key_set` / `scenario_is_test_code`

```text
                    ┌─────────────────────┐
                    │  shared internals   │
                    │  (audit module or   │
                    │   private helpers)  │
                    └──────────┬──────────┘
               ┌───────────────┼───────────────┐
               ▼               ▼               ▼
         run_audit      for_change      would_be_orphaned
         (gate)         (progress)      (archive guard)
```

**Shape of the refactor:** keep scan/projection as library-internal functions that both
`run_audit` and `for_change` call. Prefer not to duplicate the walker. Prefer not to
publicize a large grab-bag of audit internals — only `for_change` (and existing public
audit APIs) are the consumer surface.

Projection semantics must match audit’s change-introduced set:

```
| Artifact | Included keys |
| --- | --- |
| `changes/<n>/caps/.../spec.md` | all scenarios in the new cap |
| `spec.delta.md` | scenarios present after merge that were absent in base |
| deleted whole-cap delta | none |
```

Only keys with `test:code` (scenario marker overrides requirement; same rule as
`scenario_is_test_code` today) enter the snapshot.

On merge failure: record `ChangeMergeError`, skip that delta’s contribution, continue —
status is a dashboard, one bad delta should not blank the whole report.

## `status_change` coverage section

`crates/duckspec/src/cmd/status.rs` — `status_change` only.

```text
status_change today                    status_change after
───────────────────                    ───────────────────
parse change specs                     for_change(...)
marker path non-empty? → covered       linked / open from helper
else → "missing:"                      open → list (progress language)
```

```rust
// status_change — coverage block sketch
let coverage = duckpond::change_coverage::for_change(
    duckspec_root,
    project_root,
    &config,
    change_name,
)?;

// merge_errors: one visible line each, keep rendering
// linked.len() / (linked + open) as the progress fraction
// list only open keys under an "open:" (or equivalent) heading
// never list a linked key as open/missing
```

**Removed for change status:** the local `spec_coverage` / `delta_new_coverage` path that
treats marker `backlinks: Vec` as coverage. Those helpers may remain for other status
targets only if still needed (e.g. single-file `status_spec`); they must not drive
`status <change>`.

**Unchanged:** proposal/design/delta counts, step checkbox summaries, review listing,
global project status.

**Config loading:** status gains the same `Config::load` + project_root derivation audit
already uses so the scan boundary matches (`audit/scan-boundary`).

## Decisions

- **Dedicated `for_change` helper, not `run_audit`** — status needs progress facts only.
  Alternatives: call `run_audit(AuditScope::Change)` and re-render (rejected: wrong
  purpose, pulls pending/error gates, heavier, couples dashboard to gate); duplicate scan
  in the CLI (rejected: two walkers drift).

- **Source `@spec` resolution is ground truth** — not marker path lists. Alternatives:
  count marker paths (status today; rejected: false “missing” with good source links);
  require both marker path and source (rejected: still false-negatives mid-flow; audit
  already treats source as sufficient).

- **Change-scoped status only** — global `ds status` and single-file `status_spec` /
  `status_spec_delta` stay out of this change unless they already share the broken path
  through `status_change`. Alternatives: rewrite all status coverage surfaces (rejected:
  scope creep; proposal limited to `status <change>`).

- **Soft progress labels for open work** — unlinked scenarios are “open” progress, not
  defect language. No step-checkbox pending/error split (that remains audit-only).

## Risks

- **Source scan cost on large trees** → reuse the same roots/excludes as audit (`Config`);
  status already does non-trivial work per change; if needed later, cache is a follow-up,
  not part of this change.

- **Merge errors hide some scenarios** → surface each merge error as a visible line
  (status already does for deltas) and still report coverage for everything that projected
  successfully.

- **Key string mismatch (cap path / names)** → use the same `ScenarioKey` construction and
  `scenario_is_test_code` rule as audit so status and audit agree when a link is good.

## Open questions

- none
