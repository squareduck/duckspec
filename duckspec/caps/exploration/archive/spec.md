# Exploration archive

Duckboard can soft-archive explorations without deleting their chats, hide them from live
lists while archived, and expose a single hover control that archives when live and
removes when archived.

## Requirement: Soft archive state

An exploration is live when it has no archive stamp and archived when it has one.
Archiving an exploration SHALL record an archive time and SHALL NOT delete its chat
sessions. An exploration loaded without an archive stamp SHALL be treated as live.

> test: code

### Scenario: Live exploration has no archive stamp

- **GIVEN** a newly created exploration
- **WHEN** its archive state is read
- **THEN** it has no archive stamp

> test: code
> - crates/duckboard/src/chat_store.rs:1437

### Scenario: Archiving stamps archive time and keeps chats

- **GIVEN** a live exploration with at least one chat session
- **WHEN** the exploration is archived
- **THEN** the exploration has an archive stamp
- **AND** its chat sessions remain available under its scope

> test: code
> - crates/duckboard/src/chat_store.rs:1445

### Scenario: Missing stamp loads as live

- **GIVEN** persisted exploration data with no archive stamp field
- **WHEN** explorations are loaded
- **THEN** that exploration is treated as live

> test: code
> - crates/duckboard/src/chat_store.rs:1471

## Requirement: Live list membership

Live lists of explorations (Change picker and Dashboard Explorations) SHALL include only
non–idea-owned explorations that are not archived. Archiving a non–idea-owned exploration
SHALL remove it from those live lists; a live non–idea-owned exploration SHALL remain
listed.

> test: code

### Scenario: Archived non–idea-owned exploration is absent from live lists

- **GIVEN** a non–idea-owned exploration that is archived
- **WHEN** live exploration lists are built
- **THEN** that exploration does not appear on those lists

> test: code
> - crates/duckboard/src/area/change.rs:2600

### Scenario: Live non–idea-owned exploration remains on live lists

- **GIVEN** a non–idea-owned exploration that is live
- **WHEN** live exploration lists are built
- **THEN** that exploration appears on those lists

> test: code
> - crates/duckboard/src/area/change.rs:2609

## Requirement: Hover control by state

On the Change list, the single hover leading control for an exploration SHALL archive the
exploration when it is live, and SHALL remove the exploration (deleting its scope and chat
data) when it is archived. Remove SHALL require an arm-then-commit when the exploration
has sessions, and SHALL commit on the first activation when it has none.

> test: code

### Scenario: Live exploration hover control archives

- **GIVEN** a live exploration shown on the Change list
- **WHEN** its hover control is activated
- **THEN** the exploration is archived
- **AND** its chat sessions remain available under its scope

> test: code
> - crates/duckboard/src/area/change.rs:2620

### Scenario: Archived exploration hover control removes

- **GIVEN** an archived exploration shown on the Change list
- **AND** the remove control is ready to commit
- **WHEN** its hover control is activated
- **THEN** the exploration is no longer retained
- **AND** its chat sessions are deleted

> test: code
> - crates/duckboard/src/area/change.rs:2667

### Scenario: Remove with sessions requires arm then commit

- **GIVEN** an archived exploration with at least one chat session
- **WHEN** its hover control is activated once
- **THEN** the exploration is still retained
- **AND** a second activation of the control removes it

> test: code
> - crates/duckboard/src/area/change.rs:2715

### Scenario: Remove with no sessions commits without arm

- **GIVEN** an archived exploration with no chat sessions
- **WHEN** its hover control is activated once
- **THEN** the exploration is no longer retained

> test: code
> - crates/duckboard/src/area/change.rs:2771
