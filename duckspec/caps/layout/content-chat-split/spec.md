# Content / chat column split

Default equal free-space width for the content and interaction columns until the user
customizes via grip drag; absolute width after that; full remaining width for the
interaction column when content is hidden.

## Requirement: Uncustomized equal width

While an interaction panel is uncustomized and the content column is shown, the
interaction column width SHALL equal half of free horizontal space: window width minus
fixed left chrome (sidebar, list column, and their vertical dividers) and the interaction
handle. That half SHALL be floored at the minimum panel width. Uncustomized half-width
SHALL NOT be capped by a fixed maximum below free space (so a true half is allowed on wide
windows). Window resize, door open, programmatic force-show of the panel, and other
default layout refresh while uncustomized SHALL recompute this equal width from the
current window. Constructing a new interaction panel with a known window width SHALL
initialize the uncustomized width to half of free space for that window (not only a fixed
default window size). Force-showing a panel SHALL NOT mark it customized.

> test: code

### Scenario: Default half of free space

- **GIVEN** an uncustomized interaction panel

- **AND** the content column is shown

- **AND** a window width whose free space is large enough for a half above the minimum
  panel width

- **WHEN** the interaction column width is resolved for layout

- **THEN** the width equals half of free space for that window

> test: code
> - crates/duckboard/src/area/interaction.rs:2421

### Scenario: Resize rebalances to half free space

- **GIVEN** an uncustomized interaction panel

- **AND** the content column is shown

- **WHEN** the window width changes to a new size whose free space still allows a half
  above the minimum panel width

- **THEN** the interaction column width equals half of free space for the new window

> test: code
> - crates/duckboard/src/area/interaction.rs:2440

### Scenario: Half floors at minimum panel width

- **GIVEN** an uncustomized interaction panel
- **AND** a window width whose free space is less than twice the minimum panel width
- **WHEN** the interaction column width is resolved for layout
- **THEN** the width equals the minimum panel width

> test: code
> - crates/duckboard/src/area/interaction.rs:2454

### Scenario: Half may exceed the old fixed max width

- **GIVEN** an uncustomized interaction panel
- **AND** a window width whose free space is more than twice 800 logical pixels
- **WHEN** the interaction column width is resolved for layout
- **THEN** the width equals half of free space
- **AND** the width is greater than 800 logical pixels

> test: code
> - crates/duckboard/src/area/interaction.rs:2472

### Scenario: Programmatic open rebalances to half free space

- **GIVEN** an uncustomized interaction panel whose width was set for a different window
  size

- **AND** the content column is shown

- **AND** a current window width whose free space is large enough for a half above the
  minimum panel width

- **WHEN** the panel is force-shown without a door open

- **THEN** the interaction column width equals half of free space for the current window

- **AND** the panel remains uncustomized

> test: code
> - crates/duckboard/src/area/interaction.rs:2655

### Scenario: Panel created for a known window starts at half free space

- **GIVEN** a window width whose free space is large enough for a half above the minimum
  panel width

- **AND** that width is not the fixed default window size

- **WHEN** a new uncustomized interaction panel is constructed for that window

- **THEN** the interaction column width equals half of free space for that window

> test: code
> - crates/duckboard/src/area/interaction.rs:2630

## Requirement: Grip customization

The first grip-driven width change on an interaction panel SHALL mark that panel
customized and set its width to the chosen absolute value. While customized, window resize
SHALL keep that absolute width (no equal rebalance). Opening, closing, or
collapsing/restoring the content column SHALL NOT mark the panel customized by itself.

> test: code

### Scenario: First grip width change locks absolute width

- **GIVEN** an uncustomized interaction panel

- **WHEN** the grip sets the interaction column to a chosen absolute width

- **THEN** the panel is customized

- **AND** the interaction column width equals that absolute width while the content column
  is shown

> test: code
> - crates/duckboard/src/area/interaction.rs:2490

### Scenario: Resize after lock keeps absolute width

- **GIVEN** a customized interaction panel with a remembered absolute width
- **AND** the content column is shown
- **WHEN** the window width changes
- **THEN** the interaction column width remains that absolute width

> test: code
> - crates/duckboard/src/area/interaction.rs:2511

### Scenario: Open/close and content collapse do not lock

- **GIVEN** an uncustomized interaction panel

- **WHEN** the panel is closed and opened again

- **AND** the content column is collapsed and restored without a grip width change

- **THEN** the panel remains uncustomized

- **AND** the interaction column width still equals half of free space for the current
  window while the content column is shown

> test: code
> - crates/duckboard/src/area/interaction.rs:2580

## Requirement: Content-hidden fill

When the content column is not shown (no open tabs in a three-column area, or content
collapsed), a visible interaction column SHALL take the remaining horizontal space after
fixed left chrome (and the handle when present), not the equal-split absolute width.

In any three-column area, the content column SHALL NOT be shown when there are no open
tabs (neither a preview tab nor file tabs). When a list selection opens a tab, the content
column SHALL be shown again (interaction width follows equal or customized rules as when
content is visible).

> test: code

### Scenario: Interaction column fills when content column is hidden

- **GIVEN** a visible interaction panel in a three-column area

- **AND** the content column is not shown

- **WHEN** the three-column area is laid out

- **THEN** the interaction column uses the remaining width after fixed left chrome rather
  than a fixed equal-split width

> test: code
> - crates/duckboard/src/area/interaction.rs:2531

### Scenario: No open tabs hides content column

- **GIVEN** a three-column area with a visible interaction panel
- **AND** no open tabs (no preview and no file tabs)
- **AND** content is not manually collapsed
- **WHEN** the three-column area is laid out
- **THEN** the content column is not shown
- **AND** the interaction column fills the remaining width after fixed left chrome

> test: code
> - crates/duckboard/src/area/interaction.rs:2548

### Scenario: Opening a tab restores content column

- **GIVEN** a three-column area where the content column is hidden because there are no
  open tabs

- **AND** a visible interaction panel

- **WHEN** a list selection opens a tab

- **THEN** the content column is shown

- **AND** the interaction column uses its equal or customized fixed width rather than fill

> test: code
> - crates/duckboard/src/area/interaction.rs:2564
