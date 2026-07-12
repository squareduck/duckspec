# @ Content / chat column split

## @ Requirement: Uncustomized equal width

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

### + Scenario: Programmatic open rebalances to half free space

- **GIVEN** an uncustomized interaction panel whose width was set for a different window
  size

- **AND** the content column is shown

- **AND** a current window width whose free space is large enough for a half above the
  minimum panel width

- **WHEN** the panel is force-shown without a door open

- **THEN** the interaction column width equals half of free space for the current window

- **AND** the panel remains uncustomized

> test: code

### + Scenario: Panel created for a known window starts at half free space

- **GIVEN** a window width whose free space is large enough for a half above the minimum
  panel width

- **AND** that width is not the fixed default window size

- **WHEN** a new uncustomized interaction panel is constructed for that window

- **THEN** the interaction column width equals half of free space for that window

> test: code
