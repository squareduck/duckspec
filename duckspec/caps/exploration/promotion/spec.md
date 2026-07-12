# Exploration promotion

When a newly-present change directory is detected, duckboard promotes an exploration into
it only on an authoritative binding recorded when that exploration's agent created the
change — never by inferring an owner from UI focus.

## Requirement: Promotion requires an authoritative binding

A change directory detected as newly present SHALL adopt an exploration only when a
binding for that change name exists — the binding recorded when an exploration session's
agent ran `ds create change <name>`. When no binding exists — an out-of-band creation, an
unarchive, or a version-control reappearance of a directory that already existed — the
change SHALL be left standalone and no exploration SHALL be adopted, irrespective of which
exploration is currently selected.

> test: code

### Scenario: Bound change adopts its originating exploration

- **GIVEN** an exploration whose agent created a change by name
- **AND** that change's directory is detected as newly present
- **WHEN** promotion is evaluated
- **THEN** the exploration is promoted into that change
- **AND** the exploration's chat sessions are accessible under the change's scope

> test: code
> - crates/duckboard/src/main.rs:5878

### Scenario: Unbound change adopts no exploration

- **GIVEN** a change directory detected as newly present with no binding for its name
- **AND** an unrelated exploration is currently selected
- **WHEN** promotion is evaluated
- **THEN** no exploration is promoted into the change
- **AND** the selected exploration's chat sessions remain under their own scope

> test: code
> - crates/duckboard/src/main.rs:5910

## Requirement: Bindings are single-use

A binding SHALL be consumed by the promotion it authorizes, so that a later detection of
the same change directory — such as a version-control reappearance after the change was
already promoted — does not promote an exploration again.

> test: code

### Scenario: A consumed binding does not re-promote on reappearance

- **GIVEN** a change whose binding was already consumed by promoting its exploration
- **WHEN** that change's directory is detected as newly present again
- **THEN** no exploration is promoted into the change

> test: code
> - crates/duckboard/src/main.rs:6000

## Requirement: Chat focus after bound promotion

When a newly-present change directory is promoted into from a binding, duckboard SHALL
restore keyboard focus to the chat input after that promotion so the user can continue
typing without re-selecting the input. Detecting a newly-present change directory with no
binding SHALL NOT force chat input focus as a result of that detection alone.

> test: code

### Scenario: Bound promotion restores chat input focus

- **GIVEN** an exploration whose agent created a change by name
- **AND** that change's directory is detected as newly present
- **WHEN** promotion is evaluated
- **THEN** the exploration is promoted into that change
- **AND** the chat input has keyboard focus

> test: code
> - crates/duckboard/src/main.rs:5942

### Scenario: Unbound new change does not force chat input focus

- **GIVEN** a change directory detected as newly present with no binding for its name
- **AND** the chat input does not have keyboard focus
- **WHEN** promotion is evaluated
- **THEN** no exploration is promoted into the change
- **AND** the chat input still does not have keyboard focus

> test: code
> - crates/duckboard/src/main.rs:5972
