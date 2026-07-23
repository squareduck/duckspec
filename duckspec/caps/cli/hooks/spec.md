# Stage hooks

Project hooks under `duckspec/hooks/` customize stock stage templates: create scaffolds
position-named files, and `ds template` injects their bodies at the before/after write
placeholders.

## Requirement: Create hook

`ds create hook` SHALL scaffold exactly one position-named hook file for a stage
(`hooks/<stage>-before.md` or `hooks/<stage>-after.md`) with a stage-and-position H1 seed,
creating the `hooks/` directory when needed, and SHALL refuse when the target file already
exists or when neither or both position flags are supplied.

> test: code

### Scenario: Scaffold before and after paths with skeleton

- **GIVEN** a duckspec project whose `hooks/` directory does not yet contain files for
  stage `explore`

- **WHEN** `ds create hook explore --before` and `ds create hook explore --after` are run

- **THEN** the file `hooks/explore-before.md` is created with H1 seed `# Explore - Before`

- **AND** the file `hooks/explore-after.md` is created with H1 seed `# Explore - After`

> test: code
> - crates/duckspec/tests/create_hook.rs:22

### Scenario: Refuse when the hook file already exists

- **GIVEN** `hooks/spec-before.md` already exists
- **WHEN** `ds create hook spec --before` is run
- **THEN** the command fails without overwriting the existing file

> test: code
> - crates/duckpond/src/plan.rs:873

### Scenario: Refuse without exactly one of before or after

- **GIVEN** a duckspec project

- **WHEN** `ds create hook explore` is run with neither `--before` nor `--after`, or with
  both flags together

- **THEN** the command fails without creating a hook file

> test: code
> - crates/duckspec/tests/create_hook.rs:53

## Requirement: Template injection

When rendering a stock template, `ds template` SHALL load `hooks/<stage>-before.md` and
`hooks/<stage>-after.md` relative to the duckspec root; a non-empty trimmed body SHALL
appear under `## Before write` or `## After write` respectively, and a missing or
whitespace-only file SHALL omit that section entirely. Hook body text SHALL be emitted
verbatim (no required H1 or H1 stripping).

> test: code

### Scenario: Non-empty hooks inject under section headers

- **GIVEN** a stock template that contains `## Before write` and `## After write`
  placeholders

- **AND** non-empty hook files for that stage at `hooks/<stage>-before.md` and
  `hooks/<stage>-after.md`

- **WHEN** `ds template <stage>` is run

- **THEN** the rendered output includes `## Before write` followed by the before hook body

- **AND** the rendered output includes `## After write` followed by the after hook body

> test: code
> - crates/duckspec/src/cmd/template.rs:96

### Scenario: Missing or empty hooks drop the placeholders

- **GIVEN** a stock template that contains `## Before write` and `## After write`
  placeholders

- **AND** no before hook file, or only a whitespace-only before hook file, for the stage

- **AND** no after hook file for the stage

- **WHEN** `ds template <stage>` is run

- **THEN** the rendered output does not contain a `## Before write` section

- **AND** the rendered output does not contain a `## After write` section

> test: code
> - crates/duckspec/src/cmd/template.rs:124

### Scenario: Body without H1 is rendered verbatim

- **GIVEN** a before hook file whose body is freeform text without an H1 heading

- **WHEN** `ds template` renders that stage with the hook present

- **THEN** the freeform text appears under `## Before write` without requiring or
  stripping an H1

> test: code
> - crates/duckspec/src/cmd/template.rs:160

## Requirement: Known stages

Hook create SHALL accept only the known stage names used for scaffolding — `explore`,
`backfill`, `propose`, `design`, `spec`, `step`, `apply`, `archive`, `verify`, `review`,
`followup`, and `codex` — and SHALL reject any other stage name.

> test: code

### Scenario: Unknown stage is rejected

- **GIVEN** a stage name that is not in the known stage set
- **WHEN** `ds create hook <stage> --before` is run
- **THEN** the command fails without creating a hook file

> test: code
> - crates/duckpond/src/plan.rs:866
