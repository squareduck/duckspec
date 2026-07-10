# @ Chat obvious bubble

Auto messages: ranked lifecycle `/ds-*` action chips plus optional affirm and decline,
with key-first labels and dual-purpose ⌘↩ — independent of under-input input hints, and
shown only when the global auto messages setting is enabled (default on).

## @ Requirement: Chrome visibility

Obvious chrome SHALL be shown only when all of the following hold: the global auto
messages setting is enabled, the main agent turn is not in progress, the composer input is
empty, and the chrome is non-empty (at least one lifecycle option, affirm, or decline).
The auto messages setting SHALL default to enabled. When auto messages is disabled, the
chrome SHALL NOT be shown even if the other gates would allow it. A pending or settled
oneshot for under-input input hints SHALL NOT hide the chrome when those gates hold. The
chrome SHALL NOT be shown when any gate fails.

> test: code

### ~ Scenario: Idle empty composer with chrome shows chrome

- **GIVEN** auto messages enabled
- **AND** non-empty obvious chrome for the session
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is shown

> test: code

### ~ Scenario: Streaming hides chrome

- **GIVEN** auto messages enabled
- **AND** non-empty obvious chrome for the session
- **AND** an empty composer input
- **AND** a main agent turn in progress
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is not shown

> test: code

### ~ Scenario: Non-empty composer hides chrome

- **GIVEN** auto messages enabled
- **AND** non-empty obvious chrome for the session
- **AND** a non-empty composer input
- **AND** no main agent turn in progress
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is not shown

> test: code

### ~ Scenario: Empty chrome is hidden

- **GIVEN** auto messages enabled
- **AND** empty obvious chrome for the session
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is not shown

> test: code

### ~ Scenario: Oneshot pending does not hide chrome when otherwise visible

- **GIVEN** auto messages enabled
- **AND** non-empty obvious chrome for the session
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **AND** a pending reply-suggestion oneshot
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is shown

> test: code

### + Scenario: Auto messages disabled hides chrome

- **GIVEN** auto messages disabled
- **AND** non-empty obvious chrome for the session
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is not shown

> test: code

### + Scenario: Default auto messages setting is enabled

- **GIVEN** application config defaults
- **WHEN** the auto messages setting is read
- **THEN** it is enabled

> test: code
