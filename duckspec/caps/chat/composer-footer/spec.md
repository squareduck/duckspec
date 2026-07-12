# Chat composer footer

Rules for the meta strip under the chat prompt: when the resend hint appears, how context
usage is shown by fill heat, and the short closed model label.

Rules for the meta strip under the chat prompt: when the resend hint appears, how context
usage is shown by fill heat, the short closed model label, and the Missing closed label
when the effective model is not available.

## Requirement: Resend hint only for unresumable stored session

The resend-history hint SHALL appear only when the transcript is non-empty **and** a
stored agent session id exists **and** that id is not resumable for the effective harness.
The hint SHALL NOT appear when the transcript is empty, when a stored id is resumable for
the effective harness, or when no agent session id is stored.

> test: code

### Scenario: Hint shown when stored session is unresumable

- **GIVEN** a chat with a non-empty transcript
- **AND** a stored agent session id that is not resumable for the effective harness
- **WHEN** the composer footer is rendered
- **THEN** the resend-history hint is shown

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2261

### Scenario: Hint hidden when stored session is resumable

- **GIVEN** a chat with a non-empty transcript
- **AND** a stored agent session id that is resumable for the effective harness
- **WHEN** the composer footer is rendered
- **THEN** the resend-history hint is not shown

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2278

### Scenario: Hint hidden when transcript is empty

- **GIVEN** a chat with an empty transcript
- **WHEN** the composer footer is rendered
- **THEN** the resend-history hint is not shown

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2295

### Scenario: Hint hidden when no stored agent session id

- **GIVEN** a chat with a non-empty transcript
- **AND** no stored agent session id
- **WHEN** the composer footer is rendered
- **THEN** the resend-history hint is not shown

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2311

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
> - crates/duckboard/src/widget/agent_chat.rs:2327

### Scenario: Hot fill shows used, max, and percentage

- **GIVEN** a known context window
- **AND** used tokens such that fill is at least 75%
- **WHEN** the usage readout is formatted
- **THEN** the readout includes used tokens, the window max, and the fill percentage

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2341

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
> - crates/duckboard/src/widget/agent_chat.rs:2353

## Requirement: Missing closed model label

When the effective model for the chat is not available, the closed model control SHALL
show the label `Missing` instead of a model display name.

> test: code

### Scenario: Closed label is Missing when the effective model is not available

- **GIVEN** an effective model that is not available
- **WHEN** the closed model control label is built
- **THEN** the label is `Missing`

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2373
