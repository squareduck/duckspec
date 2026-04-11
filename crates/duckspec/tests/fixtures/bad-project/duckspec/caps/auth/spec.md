# Authentication

Allows users to sign in.

## Login flow

This H2 is missing the 'Requirement: ' prefix.

## Requirement: Email-password login

The system SHALL authenticate users.

> test: code

### Valid credentials

This H3 is missing the 'Scenario: ' prefix.

### Scenario: Invalid password

- **WHEN** the user submits a wrong password
- **THEN** the system rejects the login

#### Too deep

This H4 heading is not allowed in spec files.

## Requirement: Empty requirement

## Requirement: Bad: colons

Not allowed in names.

> test: code

### Scenario: No when or then

- **GIVEN** something exists

### Scenario: Out of order GWT

- **THEN** result first
- **WHEN** action second

### Scenario: Bad keyword

- **SOMETIMES** this is not a GWT keyword
- **WHEN** something
- **THEN** something

### Scenario: Unexpected content

Some paragraph that shouldn't be here.

- **WHEN** something
- **THEN** something

### Scenario: Missing marker

No test marker and requirement has none to inherit.

- **WHEN** something
- **THEN** something
