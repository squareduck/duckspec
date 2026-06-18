# Archive backlink guard

Before `ds archive` finalizes a change into the main capabilities, it checks whether the
projected post-archive specs would leave any currently-resolving `@spec` backlink
unresolved, and refuses to write unless explicitly overridden.

## Requirement: Orphan detection

The guard SHALL compare backlink resolution before and after the projected archive and
identify source `@spec` backlinks that resolve against the current capabilities but would
not resolve once the archive lands. It SHALL attribute only the orphans the archive itself
causes — a backlink that is already unresolved before the archive SHALL NOT be reported.
Only capability spec changes affect the result; doc changes SHALL NOT produce or clear
orphans.

> test: code

### Scenario: Archiving a change that removes a backlinked scenario flags the backlink

- **GIVEN** a change whose archive would remove a scenario from a capability spec
- **AND** a source backlink that resolves to that scenario today
- **WHEN** the guard evaluates the projected archive
- **THEN** the guard reports the backlink, naming its source file

> test: code
> - crates/duckpond/tests/audit.rs:197

### Scenario: An archive that preserves every backlinked scenario reports no orphans

- **GIVEN** a change whose archive adds or renames capabilities but keeps every scenario
  that a source backlink resolves to

- **WHEN** the guard evaluates the projected archive

- **THEN** the guard reports no orphans

> test: code
> - crates/duckpond/tests/audit.rs:224

### Scenario: A backlink already unresolved before the archive is not attributed to it

- **GIVEN** a source backlink that does not resolve against the current capabilities
- **AND** a change whose archive does not introduce the scenario it points to
- **WHEN** the guard evaluates the projected archive
- **THEN** the guard does not report that backlink

> test: code
> - crates/duckpond/tests/audit.rs:254

## Requirement: Refusal and override

When the guard detects orphans, `ds archive` SHALL abort before writing any capability
files or moving the change to the archive, and SHALL name the offending source files. The
working tree SHALL be left exactly as it was. The `--allow-orphans` flag SHALL downgrade
the refusal to a warning and allow the archive to complete.

> test: code

### Scenario: Refusal leaves the capabilities and change untouched

- **GIVEN** a change whose archive would orphan a live backlink
- **WHEN** `ds archive` runs without `--allow-orphans`
- **THEN** the command fails, naming the offending source files
- **AND** the capability files are unchanged
- **AND** the change still exists under changes rather than archive

> test: code
> - crates/duckspec/tests/archive.rs:80

### Scenario: allow-orphans completes the archive with a warning

- **GIVEN** a change whose archive would orphan a live backlink
- **WHEN** `ds archive` runs with `--allow-orphans`
- **THEN** a warning naming the offending source files is emitted
- **AND** the archive completes

> test: code
> - crates/duckspec/tests/archive.rs:113
