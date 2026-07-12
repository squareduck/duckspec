# Post-implementation review: new session chat ghost migration

Reviewed `new-session-chat-ghost-migration` end-to-end after both steps. The sticky
inheritance design is sound, code and specs match it, and nothing here blocks archive.

## Scope

Post-implementation: full chain to code.

- `proposal.md`, `design.md`

- `caps/chat/default-prompts` spec/doc deltas

- Steps `01-inherited-next-action-list-resolution`, `02-new-session-next-action-seed`

- Code: `crates/duckboard/src/default_prompts.rs`, `area/interaction.rs`,
  `area/change.rs`, `area/ideas.rs`

- `ds audit` / `ds check` clean; inheritance-related unit tests green

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
```

No findings.

## Verdict

**Archive-ready.** The problem is real and the approach is the right size: a sticky
ephemeral field, pure list priority (inherited → bootstrap → trailing next), and one seed
helper on the existing `Msg::NewSession` path. Spec/doc deltas and six `@spec` tests lock
list resolution, empty-donor bootstrap, and first-action index. Design risks (stale donor
tokens, no persistence) stay accepted non-goals, not gaps. Optional polish only — not
freeze blockers.
