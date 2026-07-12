# Idea reconciliation

Keeps a change-linked idea's state in sync with the lifecycle of its change — archiving
it, with the reason recorded, when the change is archived or vanishes — and reports each
relocation so the selection and open editor follow.

## Requirement: Change-linked drift classification

Reconciliation SHALL archive a change-state idea whose linked change has been archived,
recording the reason as *via-change*. It SHALL archive a change-state idea whose linked
change no longer exists — present among neither the active nor the archived changes —
recording the reason as *orphaned*. An idea whose linked change is still active, an idea
already in the archive, and an idea with no linked change SHALL be left untouched; in
particular, an already-archived idea SHALL retain its existing archive reason.

> test: code

### Scenario: Linked change archived classifies the idea as via-change

- **GIVEN** a change-state idea linked to a change
- **AND** that change is among the archived changes
- **WHEN** reconciliation runs
- **THEN** the idea is archived
- **AND** its recorded archive reason is via-change

> test: code
> - crates/duckboard/src/idea_store.rs:820

### Scenario: Linked change gone classifies the idea as orphaned

- **GIVEN** a change-state idea linked to a change
- **AND** that change is among neither the active nor the archived changes
- **WHEN** reconciliation runs
- **THEN** the idea is archived
- **AND** its recorded archive reason is orphaned

> test: code
> - crates/duckboard/src/idea_store.rs:834

### Scenario: Active linked change leaves the idea unchanged

- **GIVEN** a change-state idea linked to a change
- **AND** that change is among the active changes
- **WHEN** reconciliation runs
- **THEN** the idea remains a change-state idea

> test: code
> - crates/duckboard/src/idea_store.rs:845

### Scenario: Already-archived idea keeps its archive reason

- **GIVEN** an archived idea whose recorded reason is manual
- **WHEN** reconciliation runs
- **THEN** the idea remains archived
- **AND** its recorded archive reason is still manual

> test: code
> - crates/duckboard/src/idea_store.rs:856

## Requirement: Relocation reporting

Reconciliation SHALL report each idea it relocated, naming both the former location and
the new location. A reconciliation that relocates no idea SHALL report no relocations.

> test: code

### Scenario: An archiving relocation is reported with source and destination

- **GIVEN** a change-state idea whose linked change has been archived
- **WHEN** reconciliation runs
- **THEN** the reported relocations include one for that idea
- **AND** it names the idea's former location and its new location

> test: code
> - crates/duckboard/src/idea_store.rs:875

### Scenario: A no-op reconciliation reports no relocations

- **GIVEN** ideas whose linked changes are all still active
- **WHEN** reconciliation runs
- **THEN** no relocations are reported

> test: code
> - crates/duckboard/src/idea_store.rs:897

## Requirement: Selection and editor follow relocations

When reconciliation relocates an idea, an active selection of that idea and an open editor
showing it SHALL continue to reference the idea at its new location rather than its former
one.

> test: code

### Scenario: The selected idea stays selected after relocation

- **GIVEN** an idea selected in the list
- **WHEN** reconciliation relocates that idea
- **THEN** the selection references the idea at its new location

> test: code
> - crates/duckboard/src/area/ideas.rs:1164

### Scenario: The open idea editor tracks the idea after relocation

- **GIVEN** an idea open in the editor
- **WHEN** reconciliation relocates that idea
- **THEN** the editor continues to show that idea
- **AND** its label reflects the idea's current title

> test: code
> - crates/duckboard/src/area/ideas.rs:1181

## Requirement: Reconciliation on archival detection

Reconciliation SHALL run whenever change archival is detected during a session, not only
when a project is first opened, so that an idea whose change is archived mid-session is
reconciled without waiting for a restart.
