# Cover session-scroll scenarios

Unit-test pure policy helpers and stick-flag outcomes for every `chat/session-scroll`
`test: code` scenario.

## Prerequisites

- [x] @step session-scroll-policy-in-update-wrapper

## Tasks

- [x] 1. Extract or expose pure helpers (identity equality, open/switch classification,
         post-update scroll action selection) so tests do not need a full iced runtime

- [x] 2. @spec chat/session-scroll Open and switch show latest: Intentional session open or switch lands at latest

- [x] 3. @spec chat/session-scroll Open and switch show latest: Stick-to-bottom engages on open or switch

- [x] 4. @spec chat/session-scroll Area navigation restores viewport: Area change restores remembered mid-history

- [x] 5. @spec chat/session-scroll Area navigation restores viewport: Area change keeps stick-to-bottom when that was the prior intent

- [x] 6. @spec chat/session-scroll Layout preserve stays within session identity: Same session keeps viewport across layout-affecting update

- [x] 7. @spec chat/session-scroll Layout preserve stays within session identity: Session identity change does not apply prior session offset
