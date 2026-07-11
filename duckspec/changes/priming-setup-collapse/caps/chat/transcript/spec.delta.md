# @ Chat transcript

## @ Requirement: Collapse defaults

Live Thinking and live Activity segments SHALL start expanded. A Thinking segment SHALL
auto-collapse when a following Answer segment appears or the turn completes, unless the
user has toggled that segment. An Activity segment SHALL auto-collapse when the turn
settles (following Answer or turn complete), unless the user has toggled it. On reload of
a finished turn, Thinking and Activity SHALL start collapsed.

A User segment whose message is marked as the synthetic first-turn priming inject SHALL
start collapsed (including on reload). A non-priming User segment SHALL remain expanded.
Syncing collapse state SHALL NOT force-collapse a priming User segment the user has
expanded; re-hide after a temporary expand is a separate timed path, not the Thinking /
Activity settle rule. Ordinary User, Answer, and System segments remain non-collapsible
except for this priming User case.

> test: code

### + Scenario: Priming Setup starts collapsed

- **GIVEN** a session whose first user message is the synthetic priming inject
- **AND** a later non-priming user message exists
- **WHEN** the transcript collapse state is synced
- **THEN** the priming User segment is collapsed
- **AND** the non-priming User segment is not collapsed

> test: code

### + Scenario: User-expanded priming is not force-collapsed by sync

- **GIVEN** a priming User segment the user has expanded
- **WHEN** collapse state is synced again without a timed re-collapse
- **THEN** the priming User segment remains expanded

> test: code

### + Scenario: Timed re-collapse forces priming collapsed

- **GIVEN** a priming User segment that is currently expanded
- **WHEN** the priming re-collapse path runs for that segment
- **THEN** the priming User segment is collapsed

> test: code

## @ Requirement: Segment presentation

Collapsed Thinking SHALL label by line count (no duration). Collapsed Activity SHALL
summarize as a count plus sample tool names. Expanded Activity SHALL show one quiet row
per tool (status + summary) with truncated output under the row when present — group
expand only, with no nested per-tool expand state.

A collapsed priming User segment SHALL label with a Setup prefix and that segment's line
count. An expanded priming User segment MAY use a short Setup header with a chevron; its
body SHALL remain readable as user-card content when open.

> test: code

### + Scenario: Priming collapsed label uses Setup and line count

- **GIVEN** a priming User segment whose body has a known number of lines
- **WHEN** the collapsed label for that segment is produced
- **THEN** the label includes `Setup`
- **AND** the label includes that line count

> test: code
