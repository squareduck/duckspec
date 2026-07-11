# Scope lifecycle bootstrap

Resolve empty-session next-action bootstrap from session scope kind (exploration →
`ds-explore`, change → first lifecycle option from facts) so the composer ghost works
without agent input hints.

## Tasks

- [x] 1. Add `AgentSession::lifecycle_bootstrap` that returns `Some("ds-explore")` for
         exploration, `scope_facts.next_command` for change, and `None` for caps/codex

- [x] 2. Wire `refresh_next_actions` to use `lifecycle_bootstrap` instead of only
         `scope_facts.next_command`

- [x] 3. @spec chat/default-prompts Next-action list: Empty exploration session seeds explore

- [x] 4. @spec chat/default-prompts Next-action list: Empty change session with unfinished steps seeds apply

- [x] 5. @spec chat/default-prompts Agent input hints gate: Empty-session next actions remain when agent input hints disabled
