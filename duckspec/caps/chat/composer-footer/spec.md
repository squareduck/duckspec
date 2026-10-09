# Chat composer footer

Rules for the meta strip under the chat prompt: when the resend hint appears, how context
usage is shown by fill heat, the closed model label including Missing, and the effort
control on a model that has one.

The effort control shows the catalog default until the chat pins a level, and a main send
uses that resolved level.

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
> - crates/duckboard/src/widget/agent_chat.rs:2482

### Scenario: Hint hidden when stored session is resumable

- **GIVEN** a chat with a non-empty transcript
- **AND** a stored agent session id that is resumable for the effective harness
- **WHEN** the composer footer is rendered
- **THEN** the resend-history hint is not shown

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2499

### Scenario: Hint hidden when transcript is empty

- **GIVEN** a chat with an empty transcript
- **WHEN** the composer footer is rendered
- **THEN** the resend-history hint is not shown

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2516

### Scenario: Hint hidden when no stored agent session id

- **GIVEN** a chat with a non-empty transcript
- **AND** no stored agent session id
- **WHEN** the composer footer is rendered
- **THEN** the resend-history hint is not shown

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2532

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
> - crates/duckboard/src/widget/agent_chat.rs:2548

### Scenario: Hot fill shows used, max, and percentage

- **GIVEN** a known context window
- **AND** used tokens such that fill is at least 75%
- **WHEN** the usage readout is formatted
- **THEN** the readout includes used tokens, the window max, and the fill percentage

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2562

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
> - crates/duckboard/src/widget/agent_chat.rs:2574

## Requirement: Missing closed model label

When the effective model for the chat is not available, the closed model control SHALL
show the label `Missing` instead of a model display name.

> test: code

### Scenario: Closed label is Missing when the effective model is not available

- **GIVEN** an effective model that is not available
- **WHEN** the closed model control label is built
- **THEN** the label is `Missing`

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2595

## Requirement: Effort control

The footer offers the selected model's effort scale, and the chat's pin is the level a
main send uses.

> test: code

### Scenario: Effort control is shown between the model and the usage readout

- **GIVEN** an effective model that is available and whose catalog row has an effort scale

- **AND** an effective model that is Missing

- **AND** an available model whose catalog row has no effort scale

- **WHEN** the composer footer is rendered for each

- **THEN** the row with an effort scale shows the effort control between the model control
  and the usage readout

- **AND** the Missing model and the row with no effort scale show no effort control

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2610

### Scenario: A pin keeps the level the chat chose

- **GIVEN** a chat with no effort pin and a selected model that has a catalog default

- **WHEN** the effort control is shown, the user chooses an offered level including that
  default, and the catalog default later changes

- **THEN** the control shows the catalog default before a level is chosen

- **AND** the chosen level is stored as the chat's pin

- **AND** the pin remains that chosen level after the catalog default changes

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2642

### Scenario: Effort pin follows the preferred model when defaults are stamped

- **GIVEN** a chat with an effort pin

- **WHEN** session defaults are stamped with no preferred model, with the same preferred
  model whose row still offers the level, with a different preferred model, with
  the same available model whose row no longer offers the level, and with the
  same model missing from the catalog

- **THEN** no preferred model keeps the pin

- **AND** the same preferred model keeps the pin while the level is still offered

- **AND** a different preferred model clears the pin and the clear is saved

- **AND** a level missing from an available row clears the pin and the clear is saved

- **AND** the same model missing from the catalog keeps the pin and the effort control is
  hidden

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2670

### Scenario: Effort labels and menu follow the row's scale

- **GIVEN** a model whose effort levels include `xhigh` and an id outside the display map
- **WHEN** the closed effort control and its menu are built
- **THEN** `xhigh` is shown as `Xhigh`
- **AND** the id outside the display map is shown unchanged
- **AND** the menu lists the row's levels in catalog order
- **AND** the menu has no Default row

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2717

### Scenario: A main send uses the resolved effort level

- **GIVEN** a main chat whose selected row has an effort scale and a pin
- **AND** a main chat whose selected row has an effort scale and no pin
- **AND** a main chat whose selected row has no effort scale
- **WHEN** a main-chat turn is sent for each
- **THEN** the pinned chat sends that pinned level
- **AND** the chat with no pin sends the catalog default
- **AND** the row with no effort scale sends no effort

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2742
