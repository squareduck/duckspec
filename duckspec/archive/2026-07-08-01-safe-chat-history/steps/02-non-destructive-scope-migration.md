# Non-destructive scope migration

Replace the two clobber sites — the on-disk `rename_scope` and the in-memory
`interactions.insert` overwrite — with merges that never drop a session.

## Prerequisites

- [ ] @step atomic-session-writes

## Tasks

- [x] 1. Add `merge_scope(from, to, project_root)` in `chat_store.rs`: for each
         `<id>.json` in the source scope, move it into the target when absent; on a
         same-id collision keep the copy with more messages and rename the loser to
         `<id>.json.orphan` rather than deleting it. Remove the source directory once
         emptied.

- [x] 2. Add `merge_sessions(into, incoming)` in `interaction.rs`: fold incoming sessions
         into `into`, skipping ids already present (keeping the fuller copy on collision),
         then re-run `reconcile_display_names`; leave `into.instance_id` untouched so its
         subscriptions survive.

- [x] 3. Route the on-disk migration in `promote_exploration` / `promote_idea_exploration`
         through `merge_scope` instead of the clobber-prone `rename_scope`.

- [x] 4. Replace the `interactions.insert(Scope::Change(name), ix)` overwrite in both
         promote paths with `merge_sessions` into the target scope's existing
         `InteractionState` when one is present; fall back to insert only when the target
         scope is absent.

- [x] 5. @spec chat/persistence Non-destructive scope migration: Migration into an occupied scope keeps both scopes' sessions

- [x] 6. @spec chat/persistence Non-destructive scope migration: Same-id collision keeps the fuller session and preserves the other
