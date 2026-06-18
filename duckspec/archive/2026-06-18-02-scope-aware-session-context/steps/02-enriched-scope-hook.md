# Enriched scope hook

Carry the change facts into `SessionScope` and render an authoritative, self-describing
Change orientation; leave non-change scopes free of change facts.

## Prerequisites

- [ ] @step change-scope-facts-and-the-phase-ladder

## Context

`CurrentScopeHook::compute` (`scope.rs`) already matches on `ScopeKind` and returns the
orientation text. The Change arm currently emits only the change name. `send_prompt_text`
(`area/interaction.rs`) builds a `SessionScope` at two sites (the priming path and the
no-session-id path) — both read `ax.scope_kind` and `ax.session.scope`; extend both to
also pass `ax.scope_facts`.

## Tasks

- [x] 1. Add `pub change_facts: Option<ChangeScopeFacts>` to `SessionScope` (`scope.rs`)

- [x] 2. Populate `change_facts` from `ax.scope_facts` at both `SessionScope` construction
         sites in `send_prompt_text`

- [x] 3. Render the Change arm from the facts: name the change, report step progress (and
         the active step's task tally when present), name the suggested next stage, and
         state that change-acting commands target this change by default — disambiguating
         only when the user names a different change

- [x] 4. Confirm the exploration, caps, and codex arms render their own orientation and
         include no change progress or next-stage

- [x] 5. @spec session/scope Change identification and authority: Orientation names the scoped change as the default command target

- [x] 6. @spec session/scope Non-change scope orientation: An exploration scope signals early-stage work with no change artifacts

- [x] 7. @spec session/scope Non-change scope orientation: A capability-tree scope carries no change facts
