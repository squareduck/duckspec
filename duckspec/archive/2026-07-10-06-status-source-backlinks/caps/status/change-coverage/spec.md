# Change status coverage

For `ds status <change>`, change-introduced `test:code` scenarios are linked or open by
resolving source `@spec` backlinks — never by marker path lists — so a good source link is
never reported as missing.

## Requirement: Source backlink is the linkage signal

A change-introduced `test:code` scenario SHALL be *linked* when at least one resolving
source `@spec` backlink targets it, and *open* when none does. A non-empty marker path
list alone SHALL NOT make the scenario linked.

> test: code

### Scenario: Resolving source backlink makes the scenario linked

- **GIVEN** a change that introduces a `test:code` scenario
- **AND** a source `@spec` backlink that resolves to that scenario
- **WHEN** change status coverage is computed
- **THEN** the scenario is linked

> test: code

### Scenario: Marker path list without a source backlink leaves the scenario open

- **GIVEN** a change that introduces a `test:code` scenario whose marker lists a path
- **AND** no source `@spec` backlink resolves to that scenario
- **WHEN** change status coverage is computed
- **THEN** the scenario is open

> test: code

### Scenario: A linked scenario is not reported as open

- **GIVEN** a change-introduced `test:code` scenario that is linked
- **WHEN** change status coverage is computed
- **THEN** the scenario is not reported as open

> test: code

## Requirement: Snapshot is change-introduced test code only

The progress snapshot SHALL include only `test:code` scenarios the change introduces — all
scenarios from new change cap specs, and scenarios present after a delta merge that were
absent in the base. Pre-existing base scenarios and non-`test:code` scenarios SHALL NOT
appear in the snapshot.

> test: code

### Scenario: New change-cap test:code scenario is included

- **GIVEN** a change with a new capability spec containing a `test:code` scenario
- **WHEN** change status coverage is computed
- **THEN** the snapshot includes that scenario

> test: code

### Scenario: Delta-introduced test:code scenario is included

- **GIVEN** a change with a spec delta that introduces a `test:code` scenario absent from
  the base cap

- **WHEN** change status coverage is computed

- **THEN** the snapshot includes that scenario

> test: code

### Scenario: Pre-existing base scenario is excluded

- **GIVEN** a base capability with a `test:code` scenario
- **AND** a change whose delta does not introduce that scenario
- **WHEN** change status coverage is computed
- **THEN** the snapshot does not include that scenario

> test: code

### Scenario: Non-test:code scenario is excluded

- **GIVEN** a change that introduces a scenario that is not `test:code`
- **WHEN** change status coverage is computed
- **THEN** the snapshot does not include that scenario

> test: code

## Requirement: Change status surfaces the partition

`ds status <change>` SHALL report open scenarios from this partition as open progress and
SHALL NOT list a linked scenario as missing or open. Linkage classification SHALL NOT
depend on step checkbox state.

> test: code

### Scenario: Open scenario appears in change status open list

- **GIVEN** a change with an open `test:code` scenario
- **WHEN** `ds status` runs for that change
- **THEN** the open scenario is listed as open progress

> test: code

### Scenario: Linked scenario does not appear as missing or open

- **GIVEN** a change with a linked `test:code` scenario
- **WHEN** `ds status` runs for that change
- **THEN** the scenario is not listed as missing
- **AND** the scenario is not listed as open

> test: code

### Scenario: Step checkbox state does not change linkage

- **GIVEN** a change-introduced `test:code` scenario with no source backlink
- **AND** a checked step task that references the scenario
- **WHEN** change status coverage is computed
- **THEN** the scenario is open
- **AND** linkage is the same as when the step task is unchecked

> test: code
