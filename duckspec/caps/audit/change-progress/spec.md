# Change audit progress

When auditing a single change, each `test:code` scenario the change introduces that has no
source backlink is classified by the completion of the step tasks that reference it — so
unimplemented work reads as in-progress while a checked-off but unlinked scenario reads as
a defect.

## Requirement: Classify unlinked scenarios by step completion

A change-scoped audit SHALL classify every `test:code` scenario introduced by the change
that has no resolving source backlink. The scenario SHALL be reported as *pending* when
none of the step tasks referencing it are checked, and as an *error* when at least one
referencing step task is checked. A scenario whose backlink resolves SHALL be reported as
neither.

> test: code

### Scenario: Unchecked referencing task is pending

- **GIVEN** a change-scoped audit
- **AND** a change-introduced `test:code` scenario with no source backlink
- **AND** the only step task referencing it is unchecked
- **WHEN** the audit runs
- **THEN** the report lists the scenario as pending
- **AND** does not count it as an error

> test: code
> - crates/duckpond/tests/audit.rs:386

### Scenario: Checked referencing task is an error

- **GIVEN** a change-scoped audit
- **AND** a change-introduced `test:code` scenario with no source backlink
- **AND** a checked step task references it
- **WHEN** the audit runs
- **THEN** the report lists the scenario as an error
- **AND** does not list it as pending

> test: code
> - crates/duckpond/tests/audit.rs:401

### Scenario: A scenario claimed by any checked task is an error

- **GIVEN** a change-scoped audit
- **AND** a change-introduced `test:code` scenario with no source backlink
- **AND** two step tasks reference it, one checked and one unchecked
- **WHEN** the audit runs
- **THEN** the report lists the scenario as an error

> test: code
> - crates/duckpond/tests/audit.rs:416

### Scenario: A backlinked scenario is neither pending nor an error

- **GIVEN** a change-scoped audit
- **AND** a change-introduced `test:code` scenario whose source backlink resolves
- **AND** a checked step task references it
- **WHEN** the audit runs
- **THEN** the report lists the scenario as neither pending nor an error

> test: code
> - crates/duckpond/tests/audit.rs:429

## Requirement: Pending scenarios do not fail the audit

The audit's error verdict SHALL exclude pending scenarios. A change whose only unlinked
`test:code` scenarios are pending SHALL report no errors; a scenario classified as an
error SHALL make the audit report at least one error.

> test: code

### Scenario: A change with only pending scenarios reports no errors

- **GIVEN** a change-scoped audit
- **AND** every unlinked `test:code` scenario in the change is pending
- **WHEN** the audit runs
- **THEN** the audit reports no errors
- **AND** the pending scenarios are still listed

> test: code
> - crates/duckpond/tests/audit.rs:442

### Scenario: A checked-but-unlinked scenario makes the audit report an error

- **GIVEN** a change-scoped audit

- **AND** a change-introduced `test:code` scenario with no source backlink that a checked
  step task references

- **WHEN** the audit runs

- **THEN** the audit reports at least one error

> test: code
> - crates/duckpond/tests/audit.rs:462

## Requirement: Classification is scoped to the change audit

The pending classification SHALL apply only to a change-scoped audit. A full-project audit
SHALL report an unlinked `test:code` scenario as an error and SHALL produce no pending
scenarios.

> test: code

### Scenario: Full audit reports an unlinked caps scenario as an error, not pending

- **GIVEN** a full-project audit
- **AND** a `test:code` scenario in the main capabilities with no source backlink
- **WHEN** the audit runs
- **THEN** the report lists the scenario as an error
- **AND** produces no pending scenarios

> test: code
> - crates/duckpond/tests/audit.rs:473
