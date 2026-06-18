# Audit classification tests

Integration tests in `crates/duckpond/tests/audit.rs` covering the pending/error
classification, one per `test:code` scenario.

## Prerequisites

- [ ] @step classify-unlinked-scenarios-in-the-change-audit

## Context

Tests build a temp project tree and call
`audit::run_audit(&duckspec, root,
&config, scope)`, asserting on the returned
`AuditReport` — follow the existing patterns in `tests/audit.rs` and its `write` helper. A
scenario is *pending* when its change spec marks it `test: code` and no checked step task
references it; an *error* when a checked step task references it but no source backlink
exists; *neither* when a backlink source file references it.

Write any backlink-bearing source fixture with inline `\n` escapes (as the existing
`BACKLINK_SOURCE` const does), never as a real multi-line `@spec` comment — otherwise this
project's own audit treats the fixture as a live backlink. To make a scenario pending,
simply omit its backlink source.

## Tasks

- [x] 1. @spec audit/change-progress Classify unlinked scenarios by step completion: Unchecked referencing task is pending

- [x] 2. @spec audit/change-progress Classify unlinked scenarios by step completion: Checked referencing task is an error

- [x] 3. @spec audit/change-progress Classify unlinked scenarios by step completion: A scenario claimed by any checked task is an error

- [x] 4. @spec audit/change-progress Classify unlinked scenarios by step completion: A backlinked scenario is neither pending nor an error

- [x] 5. @spec audit/change-progress Pending scenarios do not fail the audit: A change with only pending scenarios reports no errors

- [x] 6. @spec audit/change-progress Pending scenarios do not fail the audit: A checked-but-unlinked scenario makes the audit report an error

- [x] 7. @spec audit/change-progress Classification is scoped to the change audit: Full audit reports an unlinked caps scenario as an error, not pending
