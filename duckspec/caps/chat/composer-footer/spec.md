# Chat composer footer

Rules for the meta strip under the chat prompt: when the resend hint appears, how context
usage is shown by fill heat, and the short closed model label.

## Requirement: Resend hint only when history would be resent

The resend-history hint SHALL appear only when the next send would open a fresh agent
session **and** the transcript is non-empty. The hint SHALL NOT appear when the next send
would resume a session, or when the transcript is empty.

> test: code

### Scenario: Hint shown when history would be resent

- **GIVEN** a chat with no resumable agent session
- **AND** a non-empty transcript
- **WHEN** the composer footer is rendered
- **THEN** the resend-history hint is shown

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2016

### Scenario: Hint hidden when next send would resume

- **GIVEN** a chat with a resumable agent session
- **AND** a non-empty transcript
- **WHEN** the composer footer is rendered
- **THEN** the resend-history hint is not shown

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2028

### Scenario: Hint hidden when transcript is empty

- **GIVEN** a chat with no resumable agent session
- **AND** an empty transcript
- **WHEN** the composer footer is rendered
- **THEN** the resend-history hint is not shown

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2040

## Requirement: Progressive usage readout

When context fill against the selected model's window is known and below 75%, the usage
readout SHALL show the percentage only (no absolute used or max tokens). When fill is at
least 75%, the readout SHALL include used tokens, the window max, and the percentage.

> test: code

### Scenario: Cool fill shows percentage only

- **GIVEN** a known context window
- **AND** used tokens such that fill is below 75%
- **WHEN** the usage readout is formatted
- **THEN** the readout shows the fill percentage
- **AND** the readout does not include absolute used or max token counts

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2052

### Scenario: Hot fill shows used, max, and percentage

- **GIVEN** a known context window
- **AND** used tokens such that fill is at least 75%
- **WHEN** the usage readout is formatted
- **THEN** the readout includes used tokens, the window max, and the fill percentage

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2066

## Requirement: Short closed model label

The closed model control SHALL show the model's short display name without a harness
prefix.

> test: code

### Scenario: Closed label is the model display name

- **GIVEN** a selectable model with a harness name and a short display name
- **WHEN** the closed model control label is built
- **THEN** the label is the short display name
- **AND** the label does not include a harness prefix

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2078
