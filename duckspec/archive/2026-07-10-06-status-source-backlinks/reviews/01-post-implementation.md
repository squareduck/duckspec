# Post-implementation: Change status uses source backlinks

Reviewed the full chain for `status-source-backlinks` through code. The change solves the
real false-“missing” bug with the right ground truth, stays out of audit’s job, and is
ready to archive with only small craft nits.

## Scope

Post-implementation, end-to-end:

- `proposal.md`, `design.md`

- `caps/status/change-coverage/{spec,doc}.md`

- steps 01–03 (all complete; scoped audit clean)

- `crates/duckpond/src/change_coverage.rs` and the `pub(crate)` projection/scan hooks in
  `audit.rs`

- `crates/duckspec/src/cmd/status.rs` (`status_change` only)

- `crates/duckpond/tests/change_coverage.rs`, `crates/duckspec/tests/status.rs`

## Findings

### Soft CLI assertion on linked progress — quality/minor

`crates/duckspec/tests/status.rs` accepts either an exact `1/1 scenarios linked` line or
any stderr containing `scenarios linked`. The fallback would still pass if the fraction
were wrong (e.g. `0/1 scenarios linked`) as long as the phrase appears. Assert the exact
progress line only so the presentation contract cannot regress quietly.

### Catch-all error mapping loses the original audit error — quality/minor

`ChangeCoverageError::from` maps non-`Io` `AuditError` variants into
`ChangeCoverageError::Io` with an empty path and `Error::other(to_string())`
(`crates/duckpond/src/change_coverage.rs`). Shared helpers currently only return `Io`, so
this is dead today, but the mapping would hide a future non-Io failure mode. Prefer an
exhaustive match that preserves the variant (or a transparent wrap of `AuditError`) if
that arm is kept.

## What went right

- **Right product cut.** Status remains a dashboard; audit keeps pending/error gates and
  exit codes. Proposal boundaries held.

- **Right ground truth.** Source `@spec` keys drive linked/open; marker path lists are
  ignored for `status <change>` — that is exactly the bug that bit agents.

- **Shared guts, thin API.** Reusing `build_change_scenarios` / `scan_source_files` /
  `backlink_key_set` as `pub(crate)` avoids a second walker without turning status into
  `run_audit`.

- **Caps and tests match.** Ten `test: code` scenarios are linked; library tests cover
  linkage and snapshot membership; CLI tests cover open vs linked presentation.

- **Verified on this change:** freshly built `ds status status-source-backlinks` reports
  `10/10 scenarios linked` with no open/missing list.

## Verdict

Accept and archive. The thinking is sound, the realization is faithful to proposal and
design, and the code is small and maintainable. The two quality findings are optional
polish — neither blocks long-term health if frozen as-is, but the tighter CLI assertion is
a cheap improvement if you touch the file again.
