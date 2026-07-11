# Stock CLI content

Stock harness commands, agent templates, and schema guides ship inside the `ds` binary and
drive `init`, `template`, and `schema` without a runtime source tree.

## Requirement: Stock content from the binary

The `ds` binary SHALL carry stock harness command files, agent template text, and schema
guide text. `ds template <name>` SHALL print the stock template for that name.
`ds schema <name>` SHALL print the stock schema guide for that name. `ds init <harness>`
for a supported harness SHALL install that harness's stock command files under the
harness-specific commands directory in the working tree. These operations SHALL NOT
require the source tree that built the binary to be present on disk at runtime.

> test: code

### Scenario: Known template is printed

- **GIVEN** a stock template named `explore` is carried in the binary
- **WHEN** `ds template explore` is run
- **THEN** standard output contains the stock explore template body
- **AND** the command exits successfully

> test: code

### Scenario: Known schema is printed

- **GIVEN** a stock schema guide named `proposal` is carried in the binary
- **WHEN** `ds schema proposal` is run
- **THEN** standard output contains the stock proposal schema body
- **AND** the command exits successfully

> test: code

### Scenario: Known harness commands are installed under the harness path

- **GIVEN** a supported harness `claude` whose stock command files are carried in the
  binary

- **AND** a working directory with no `.claude/commands` tree yet

- **WHEN** `ds init claude` is run in that directory

- **THEN** `.claude/commands/` contains the stock `ds-*.md` command files for that harness

- **AND** each installed file's body matches the stock command body from the binary

- **AND** the command exits successfully

> test: code

## Requirement: Clear unknown-name failures

When a caller requests a stock template, schema, or harness name that is not available,
`ds` SHALL fail with an error that names the unknown value (template, schema, or harness
as appropriate). The failure SHALL NOT surface as a missing on-disk path or generic
filesystem error for the absent stock name.

> test: code

### Scenario: Unknown template is rejected by name

- **GIVEN** no stock template named `not-a-real-template`
- **WHEN** `ds template not-a-real-template` is run
- **THEN** the command fails
- **AND** the error message identifies the unknown template name
- **AND** the error message does not report a missing filesystem path for stock content

> test: code

### Scenario: Unknown schema is rejected by name

- **GIVEN** no stock schema guide named `not-a-real-schema`
- **WHEN** `ds schema not-a-real-schema` is run
- **THEN** the command fails
- **AND** the error message identifies the unknown schema name
- **AND** the error message does not report a missing filesystem path for stock content

> test: code

### Scenario: Unknown harness is rejected by name

- **GIVEN** no supported harness named `not-a-real-harness`
- **WHEN** `ds init not-a-real-harness` is run
- **THEN** the command fails
- **AND** the error message identifies the unknown harness name
- **AND** the error message does not report a missing filesystem path for stock content

> test: code
