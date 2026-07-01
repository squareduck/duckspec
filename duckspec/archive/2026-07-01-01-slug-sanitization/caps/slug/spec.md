# Title slug

The canonical rule that converts a human title into a kebab-case slug — the single source
of truth for every duckspec filename slug.

## Requirement: Slug transformation

A title SHALL be converted to a slug by lowercasing it, preserving Unicode alphanumeric
characters, mapping every run of non-alphanumeric characters to a single `-`, and trimming
leading and trailing `-`. A title with no alphanumeric characters SHALL produce an empty
string.

> test: code

### Scenario: Words become lowercase, dash-joined tokens

- **GIVEN** the title `Implement Auth`
- **WHEN** it is slugified
- **THEN** the result is `implement-auth`

> test: code

### Scenario: A run of non-alphanumeric characters collapses to one dash

- **GIVEN** the title `Soundness & fidelity`
- **WHEN** it is slugified
- **THEN** the result is `soundness-fidelity`

> test: code

### Scenario: Leading and trailing non-alphanumeric characters are dropped

- **GIVEN** the title `-- Draft! --`
- **WHEN** it is slugified
- **THEN** the result is `draft`

> test: code

### Scenario: Unicode alphanumerics are preserved

- **GIVEN** the title `Café Résumé`
- **WHEN** it is slugified
- **THEN** the result is `café-résumé`

> test: code

### Scenario: A title with no alphanumeric characters yields an empty string

- **GIVEN** the title `!!! ---`
- **WHEN** it is slugified
- **THEN** the result is the empty string

> test: code
