# Review followups: catalog seed, recovery escape, bare slash

Close the three post-implementation gaps: seed system commands before harness discovery,
re-parse `//` escape on lost-session recovery, and stop treating double-slash forms as
bare single-slash commands.

## Prerequisites

- [x] @step kinded-catalog-and-discovery-cleanup
- [x] @step local-help-and-escape
- [x] @step completion-kind-cues

## Context

Post-implementation review findings (not yet filed under `reviews/`): (1) `chat_commands`
starts empty until `CommandsAvailable`, so local `/help` body and completion lack System
entries before the agent warms; (2) `recover_from_lost_session` re-sends transcript
display text as the agent prompt, losing `//help` → `/help`; (3)
`is_bare_slash_command("//help")` is true because only one leading `/` is stripped.

## Tasks

- [x] 1. Seed `AgentSession::from_session` `chat_commands` with
         `slash_commands::system_registry()` so System commands exist before discovery
         merge

- [x] 2. Unit test: a fresh session's `chat_commands` includes System `help` without
         waiting for `CommandsAvailable`

- [x] 3. In `recover_from_lost_session`, run the last user text through
         `parse_submit_slash`; when the route is Agent, use `prompt` (not raw display) for
         the recovery `TurnRequest` (skip recovery agent re-send for LocalHelp if that
         path can appear)

- [x] 4. Unit test or pure helper covering recovery prompt selection: input `//help` →
         agent prompt `/help`

- [x] 5. Fix `is_bare_slash_command` so a second leading `/` is not bare (`//help` →
         false; `/help` still true)

- [x] 6. Extend `is_bare_slash_command` tests for `//help` and `/help`
