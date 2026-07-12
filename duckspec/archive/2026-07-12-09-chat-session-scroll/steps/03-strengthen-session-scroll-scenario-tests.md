# Strengthen session-scroll scenario tests

Rewrite the six `@spec` tests so they drive real `State` transitions (scope pick, session
tab, area round-trip) and assert stick / offset intent on the active session via the
production policy path—not only hand-set decision-table inputs.

## Prerequisites

- [x] @step cover-session-scroll-scenarios

## Context

Review finding 1 (major): pure `chat_scroll_policy` assertions cannot fail if the wrapper
stops calling `snap_chat_to_latest`. Keep unbroken `@spec` comments; replace bodies.

## Tasks

- [x] 1. Add minimal test helpers that seed two chat scopes/sessions on `State` (or
         multi-session under one scope) with controllable `stick_to_bottom` /
         `last_chat_offset_y`, without a full project tree when possible

- [x] 2. Replace each of the six scenario tests so they either call
         `update_with_scroll_preservation` / the same policy branch production uses after
         a real message, or assert active-session stick/offset after that path—not only
         `chat_scroll_policy(true, …)` with hand-set booleans

- [x] 3. @spec chat/session-scroll Open and switch show latest: Intentional session open or switch lands at latest

- [x] 4. @spec chat/session-scroll Open and switch show latest: Stick-to-bottom engages on open or switch

- [x] 5. @spec chat/session-scroll Area navigation restores viewport: Area change restores remembered mid-history

- [x] 6. @spec chat/session-scroll Area navigation restores viewport: Area change keeps stick-to-bottom when that was the prior intent

- [x] 7. @spec chat/session-scroll Layout preserve stays within session identity: Same session keeps viewport across layout-affecting update

- [x] 8. @spec chat/session-scroll Layout preserve stays within session identity: Session identity change does not apply prior session offset
