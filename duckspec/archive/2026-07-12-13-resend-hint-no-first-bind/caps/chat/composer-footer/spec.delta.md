# @ Chat composer footer

## = Requirement: Resend hint only when history would be resent

Requirement: Resend hint only for unresumable stored session

## @ Requirement: Resend hint only for unresumable stored session

The resend-history hint SHALL appear only when the transcript is non-empty **and** a
stored agent session id exists **and** that id is not resumable for the effective harness.
The hint SHALL NOT appear when the transcript is empty, when a stored id is resumable for
the effective harness, or when no agent session id is stored.

> test: code

### = Scenario: Hint shown when history would be resent

Scenario: Hint shown when stored session is unresumable

### = Scenario: Hint hidden when next send would resume

Scenario: Hint hidden when stored session is resumable

### ~ Scenario: Hint shown when stored session is unresumable

- **GIVEN** a chat with a non-empty transcript
- **AND** a stored agent session id that is not resumable for the effective harness
- **WHEN** the composer footer is rendered
- **THEN** the resend-history hint is shown

> test: code

### ~ Scenario: Hint hidden when stored session is resumable

- **GIVEN** a chat with a non-empty transcript
- **AND** a stored agent session id that is resumable for the effective harness
- **WHEN** the composer footer is rendered
- **THEN** the resend-history hint is not shown

> test: code

### ~ Scenario: Hint hidden when transcript is empty

- **GIVEN** a chat with an empty transcript
- **WHEN** the composer footer is rendered
- **THEN** the resend-history hint is not shown

> test: code

### + Scenario: Hint hidden when no stored agent session id

- **GIVEN** a chat with a non-empty transcript
- **AND** no stored agent session id
- **WHEN** the composer footer is rendered
- **THEN** the resend-history hint is not shown

> test: code
