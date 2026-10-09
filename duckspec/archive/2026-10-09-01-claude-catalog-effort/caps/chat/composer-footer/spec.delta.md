# @ Chat composer footer

Rules for the meta strip under the chat prompt: when the resend hint appears, how context
usage is shown by fill heat, the closed model label including Missing, and the effort
control on a model that has one.

The effort control shows the catalog default until the chat pins a level, and a main send
uses that resolved level.

## + Requirement: Effort control

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

### Scenario: A pin keeps the level the chat chose

- **GIVEN** a chat with no effort pin and a selected model that has a catalog default

- **WHEN** the effort control is shown, the user chooses an offered level including that
  default, and the catalog default later changes

- **THEN** the control shows the catalog default before a level is chosen

- **AND** the chosen level is stored as the chat's pin

- **AND** the pin remains that chosen level after the catalog default changes

> test: code

### Scenario: Effort pin follows the preferred model when defaults are stamped

- **GIVEN** a chat with an effort pin

- **WHEN** session defaults are stamped with no preferred model, with the same preferred
  model whose row still offers the level, with a different preferred model, with the same
  available model whose row no longer offers the level, and with the same model missing
  from the catalog

- **THEN** no preferred model keeps the pin

- **AND** the same preferred model keeps the pin while the level is still offered

- **AND** a different preferred model clears the pin and the clear is saved

- **AND** a level missing from an available row clears the pin and the clear is saved

- **AND** the same model missing from the catalog keeps the pin and the effort control is
  hidden

> test: code

### Scenario: Effort labels and menu follow the row's scale

- **GIVEN** a model whose effort levels include `xhigh` and an id outside the display map
- **WHEN** the closed effort control and its menu are built
- **THEN** `xhigh` is shown as `Xhigh`
- **AND** the id outside the display map is shown unchanged
- **AND** the menu lists the row's levels in catalog order
- **AND** the menu has no Default row

> test: code

### Scenario: A main send uses the resolved effort level

- **GIVEN** a main chat whose selected row has an effort scale and a pin
- **AND** a main chat whose selected row has an effort scale and no pin
- **AND** a main chat whose selected row has no effort scale
- **WHEN** a main-chat turn is sent for each
- **THEN** the pinned chat sends that pinned level
- **AND** the chat with no pin sends the catalog default
- **AND** the row with no effort scale sends no effort

> test: code
