# Full-write refresh helper and tests

Extract one force-refresh write path that updates session title and exploration display
name together, and retarget the hollow refresh `@spec` tests at that path.

## Context

Review finding 2: overwrite / failed-empty / no-content tests compose pure helpers and
never call the integration that force-writes both labels. Production is
`apply_session_title_inner(..., force: true)` in `main.rs` after
`SessionTitleRefreshReady`. First-turn auto-title must stay force=false.

## Tasks

- [ ] 1. Extract a testable full-write helper (e.g. in `chat_store` or a small pure
         module) that, given a force flag and a non-empty title, updates the session title
         (honoring force) and, for exploration scopes, the matching exploration
         `display_name`; leave both unchanged on empty/whitespace or failed accept

- [ ] 2. Wire `apply_session_title_inner` to use that helper so UI and tests share one
         write path; keep auto first-turn titles on force=false

- [ ] 3. @spec exploration/list-labels Refresh retitles from the active session chat: Refresh overwrites an existing title and exploration label

- [ ] 4. @spec exploration/list-labels Refresh retitles from the active session chat: Failed or empty refresh leaves labels unchanged

- [ ] 5. @spec exploration/list-labels Refresh retitles from the active session chat: Refresh with no summarizable content leaves labels unchanged
