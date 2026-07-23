# Post-implementation review: hook before/after + caps

Clean review after both steps completed. Proposal, design, `cli/hooks` caps,
implementation, and tests align; mechanical `ds check` and
`ds audit hook-before-after-caps` both pass. No accepted findings.

## Scope

- Proposal and design (rename + new capability, no shared path helper, stock-content
  untouched)

- Change caps `cli/hooks` (`spec.md` + `doc.md`)

- Steps 01–02 (all tasks checked)

- Source: `crates/duckpond/src/plan.rs`, `crates/duckspec/src/cmd/create.rs`,
  `crates/duckspec/src/cmd/template.rs`

- Tests: plan unit hooks, `crates/duckspec/tests/create_hook.rs`, template unit injection
  suite

- Working-copy diff for this change only

## Summary

```
| # | finding | resolution | → next |
| --- | --- | --- | --- |
```

## Findings

No accepted findings.

## Outcome

The change is complete and faithful: create/plan vocabulary is `before`/`after`
end-to-end, `cli/hooks` owns create, injection, and known stages, and all seven scenarios
resolve via `@spec` backlinks with passing tests. Design exclusions (no aliases, no shared
path API, no stock-content ownership edit, no hook audit rules) hold.

Primary next route: `/ds-archive` — merge `cli/hooks` into top-level caps and archive the
change.
