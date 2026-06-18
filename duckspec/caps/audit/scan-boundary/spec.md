# Backlink scan boundary

Defines which source files the `@spec` backlink scan reads — the scan roots and the
exclusions — so the audit resolves only genuine backlinks and never flags markers it
should not have looked at.

## Requirement: Scan roots

The scan SHALL read from the `test_paths` entries in `config.toml` (each resolved relative
to the project root) when that list is non-empty, and SHALL otherwise read from the entire
project root. A configured `test_paths` entry that does not exist SHALL be skipped rather
than cause an error.

> test: code

### Scenario: Configured test_paths scope the scan

- **GIVEN** a project whose config sets `test_paths` to a single directory
- **AND** one source backlink inside that directory and another outside it
- **WHEN** the source scan runs
- **THEN** only the backlink inside the configured directory is returned

> test: code
> - crates/duckpond/src/audit.rs:1218

### Scenario: Empty test_paths scans from the project root

- **GIVEN** a project whose config sets no `test_paths`
- **AND** a source backlink anywhere under the project root
- **WHEN** the source scan runs
- **THEN** the backlink is returned

> test: code
> - crates/duckpond/src/audit.rs:1238

## Requirement: Excluded paths

Entries in the `config.toml` `exclude` list SHALL be omitted from the scan: an entry
naming a file omits that file, and an entry naming a directory omits that directory and
its whole subtree. The `exclude` value SHALL parse as an array of strings, defaulting to
empty when absent; a non-array value SHALL fail config loading with
`ConfigError::BadExclude`.

> test: code

### Scenario: Excluded file and excluded directory subtree contribute no backlinks

- **GIVEN** a project whose `exclude` list names one source file and one directory

- **AND** backlinks in the excluded file, inside the excluded directory's subtree, and in
  a non-excluded location

- **WHEN** the source scan runs

- **THEN** only the backlink in the non-excluded location is returned

> test: code
> - crates/duckpond/src/audit.rs:1254

### Scenario: Non-array exclude raises BadExclude

- **GIVEN** a `config.toml` whose `exclude` key is a string rather than an array
- **WHEN** the config is loaded
- **THEN** loading fails with `ConfigError::BadExclude`

> test: code
> - crates/duckpond/src/config.rs:179

## Requirement: Nested duckspec projects

A directory that owns its own `duckspec/caps/` SHALL be treated as a self-governing
project and skipped entirely, along with its whole subtree, without needing an `exclude`
entry — its backlinks resolve against its own specs, not the enclosing project's. Skipping
nested projects SHALL NOT suppress backlinks elsewhere in the enclosing project.

> test: code

### Scenario: A nested project is skipped while the enclosing project is still scanned

- **GIVEN** a project containing a subdirectory that owns its own `duckspec/caps/`

- **AND** a source backlink inside that nested project and another in the enclosing
  project outside it

- **WHEN** the source scan runs

- **THEN** the backlink in the nested project is not returned

- **AND** the backlink in the enclosing project is returned

> test: code
> - crates/duckpond/src/audit.rs:1283
