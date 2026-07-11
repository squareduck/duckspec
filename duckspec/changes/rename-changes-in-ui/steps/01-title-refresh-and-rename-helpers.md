# Title refresh and rename helpers

Pure exploration-label and title-refresh helpers in chat_store / apply path, with unit
coverage for rename and refresh contracts.

## Context

First-turn auto-title still uses `title_summarization_target` (first real user message)
and `apply_session_title` which no-ops when a title exists. Refresh needs a conversation
input builder and a force-overwrite apply path. Manual rename only mutates
`Exploration.display_name` + `save_explorations`.

## Tasks

- [x] 1. Add a pure helper that commits an exploration rename: non-empty trimmed text
         updates `display_name`; blank/whitespace leaves it unchanged; return whether the
         name changed

- [x] 2. @spec exploration/list-labels Manual rename updates the exploration label: Non-empty rename replaces the list label and persists

- [x] 3. @spec exploration/list-labels Manual rename updates the exploration label: Blank rename leaves the label unchanged

- [x] 4. Add `title_refresh_target` (or equivalent) that builds summarizer input from the
         active session’s non-priming conversation so later non-bare user turns are
         included when present; return `None` when nothing is summarizable

- [x] 5. @spec exploration/list-labels Refresh retitles from the active session chat: Refresh input includes later user turns when present

- [x] 6. @spec exploration/list-labels Refresh retitles from the active session chat: Refresh with no summarizable content leaves labels unchanged

- [x] 7. Extend session-title apply so a forced refresh can overwrite an existing
         non-empty title and still update the exploration `display_name` when the scope is
         an exploration; keep the existing one-shot path’s “set only if unset” behavior
         for automatic first-turn titles

- [x] 8. @spec exploration/list-labels Refresh retitles from the active session chat: Refresh overwrites an existing title and exploration label

- [x] 9. @spec exploration/list-labels Refresh retitles from the active session chat: Failed or empty refresh leaves labels unchanged
