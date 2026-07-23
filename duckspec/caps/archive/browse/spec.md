# Archive browse

Duckboard presents archived work newest-first, interleaves archived explorations with
archived changes on Change and Dashboard archived lists, and keeps archived sections
closed by default.

## Requirement: Archived change order

Archived changes SHALL be listed most recent first, using each archive folder's
date-and-counter prefix as the order key.

> test: code

### Scenario: Archived changes list most recent first

- **GIVEN** more than one archived change with distinct archive prefixes
- **WHEN** the archived change list is built
- **THEN** entries appear in descending archive-prefix order

> test: code
> - crates/duckboard/src/data.rs:554

## Requirement: Interleaved archived rows

The Change list and Dashboard Archived lists SHALL include non–idea-owned archived
explorations together with archived changes, ordered by archive date descending.
Idea-owned archived explorations SHALL NOT appear on those lists.

> test: code

### Scenario: Archived non–idea-owned explorations appear with archived changes

- **GIVEN** at least one archived change
- **AND** a non–idea-owned archived exploration
- **WHEN** the Change or Dashboard archived list is built
- **THEN** both the change and the exploration appear as rows

> test: code
> - crates/duckboard/src/area/change.rs:2857

### Scenario: Mixed archive rows order by archive date descending

- **GIVEN** archived changes and non–idea-owned archived explorations with distinct
  archive dates

- **WHEN** the archived list is built

- **THEN** all rows appear in descending archive-date order

> test: code
> - crates/duckboard/src/area/change.rs:2869

### Scenario: Idea-owned archived explorations stay off Change and Dashboard archived lists

- **GIVEN** an idea-owned exploration that is archived
- **WHEN** the Change or Dashboard archived list is built
- **THEN** that exploration does not appear as a row

> test: code
> - crates/duckboard/src/area/change.rs:2892

## Requirement: Archived section visibility

The Change list Archived section SHALL be absent only when there are no archived changes
and no listable archived explorations. The Ideas Archive section and the Change Archived
section SHALL start collapsed until the user expands them.

> test: code

### Scenario: Archived section is empty only when both kinds are empty

- **GIVEN** no archived changes
- **AND** one non–idea-owned archived exploration
- **WHEN** the Change list is built
- **THEN** the Archived section is present
- **AND** it contains that exploration

> test: code
> - crates/duckboard/src/area/change.rs:2908

### Scenario: Ideas Archive section starts collapsed

- **GIVEN** a fresh Ideas list with no user expand overrides
- **WHEN** the Ideas list is shown
- **THEN** the Archive section is collapsed

> test: code
> - crates/duckboard/src/area/ideas.rs:1206

### Scenario: Change Archived section starts collapsed

- **GIVEN** a fresh Change list with no user expand overrides
- **AND** at least one archived row to show
- **WHEN** the Change list is shown
- **THEN** the Archived section is collapsed

> test: code
> - crates/duckboard/src/area/change.rs:2923
