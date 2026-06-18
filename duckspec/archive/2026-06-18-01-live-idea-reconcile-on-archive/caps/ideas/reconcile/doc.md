# Idea reconciliation

Reconciliation keeps an idea's state consistent with the change it is linked to. An idea
flows through `Inbox -> Exploration -> Change -> Archive`; once it is attached to a
change, the change's own lifecycle can move out from under it. Reconciliation detects that
drift and archives the idea, recording why, so the board never shows an idea pinned to a
change that is no longer active.

## When it runs

Reconciliation runs against the current set of active and archived changes:

- when a project is opened, and
- whenever change archival is detected while the project is open.

The second trigger is what keeps the board live: archiving a change — for example with the
`ds archive` CLI while the board is running — reconciles the linked idea immediately,
without waiting for a restart.

Reconciliation only inspects ideas already loaded for the session. Refreshing that list
against ideas edited directly on disk is a separate concern.

## Drift outcomes

Only a change-state idea — one carrying a link to a change — can drift. The linked
change's status determines the outcome:

```text
linked change is active     → no change
linked change is archived   → idea archived, reason = via-change
linked change does not exist → idea archived, reason = orphaned
```

An idea already in the archive is never reclassified, so a reason set earlier — including
a manual archive — is preserved. An idea with no linked change has nothing to reconcile.

## Archive reasons

The reason an idea was archived is recorded so the board can show *why* it landed there:

```text
manual      archived by the user directly
via-change  archived because its linked change was archived
orphaned    archived because its linked change no longer exists
```

Reconciliation produces only `via-change` and `orphaned`. `manual` is set by the
user-driven archive action and is left untouched here.

## Following relocations

Archiving an idea moves its file from the change subtree into the archive subtree, so the
idea's location changes. Reconciliation reports every such relocation — the former and new
location of each moved idea — so callers can keep the rest of the board in sync. The list
selection and an open idea editor use these reports to follow a reconciled idea to its new
location instead of pointing at the stale one.
