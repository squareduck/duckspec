# Chat session scroll

When the active chat session is opened or switched, the transcript shows the latest
content; when only the area changes, that session’s remembered viewport is restored.
Layout reflow preserves scroll only while session identity is unchanged.

## Requirement: Open and switch show latest

When the active chat session identity becomes a different session through an intentional
open or switch — including first time a session becomes the visible chat for a scope pick,
session-tab selection, new session, or clear session — the chat transcript viewport SHALL
show the latest content (scrolled to the end). Stick-to-bottom SHALL be engaged for that
session so subsequent live follow behaves as if the user is at the bottom. Re-selecting
the already-active same session SHALL NOT force a jump to latest solely because of that
re-select.

> test: code

### Scenario: Intentional session open or switch lands at latest

- **GIVEN** a chat session whose transcript extends beyond the viewport

- **AND** a different session is about to become the active chat through an intentional
  open or switch

- **WHEN** that open or switch completes

- **THEN** the chat transcript viewport shows the latest content

> test: code
> - crates/duckboard/src/main.rs:6369

### Scenario: Stick-to-bottom engages on open or switch

- **GIVEN** an intentional open or switch that makes a different session the active chat
- **WHEN** that open or switch completes
- **THEN** stick-to-bottom is engaged for the newly active session

> test: code
> - crates/duckboard/src/main.rs:6400

## Requirement: Area navigation restores viewport

When only the active area changes and the target area’s already-active chat session
becomes visible again, the system SHALL restore that session’s remembered viewport:
stick-to-bottom when that was the prior intent, otherwise the last remembered scroll
offset. Area navigation alone SHALL NOT force latest when the remembered intent was
mid-history.

> test: code

### Scenario: Area change restores remembered mid-history

- **GIVEN** an active chat session scrolled away from the bottom with stick-to-bottom
  disengaged

- **AND** a remembered scroll offset for that session

- **WHEN** the user navigates to another area and back without changing that session’s
  identity

- **THEN** the chat transcript viewport is restored to the remembered offset

- **AND** stick-to-bottom remains disengaged

> test: code
> - crates/duckboard/src/main.rs:6431

### Scenario: Area change keeps stick-to-bottom when that was the prior intent

- **GIVEN** an active chat session with stick-to-bottom engaged

- **WHEN** the user navigates to another area and back without changing that session’s
  identity

- **THEN** stick-to-bottom remains engaged

- **AND** the chat transcript viewport shows the latest content

> test: code
> - crates/duckboard/src/main.rs:6470

## Requirement: Layout preserve stays within session identity

Layout-affecting updates that do not change the active chat session identity SHALL
preserve the current session’s scroll intent (stick-to-bottom or last offset). When the
active chat session identity does change, the system SHALL NOT apply the previous
session’s scroll offset or stick intent as the post-update layout preservation for the new
session.

> test: code

### Scenario: Same session keeps viewport across layout-affecting update

- **GIVEN** an active chat session with a stable scroll intent (mid-history or
  stick-to-bottom)

- **WHEN** a layout-affecting update runs that does not change the active chat session
  identity

- **THEN** that session’s scroll intent is preserved after the update

> test: code
> - crates/duckboard/src/main.rs:6507

### Scenario: Session identity change does not apply prior session offset

- **GIVEN** an active chat session with a non-zero remembered scroll offset

- **WHEN** an update makes a different session the active chat

- **THEN** the previous session’s scroll offset is not applied as the new session’s
  layout-preserved viewport

> test: code
> - crates/duckboard/src/main.rs:6537
