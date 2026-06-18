# Live idea reconciliation on change archival

Auto-archive an idea the moment its linked change is archived during a running session —
not only on the next app restart — and make the reconcile pass report its file moves so
the open tab and list selection follow.

## Motivation

Ideas linked to a change are supposed to auto-archive when that change is archived,
landing in the `Archive` state with a sub-kind that records *why*: `ViaChange` (the change
was archived) or `Orphaned` (the change directory vanished). The drift-detection logic
that does this — `reconcile` -> `drift_target` in `idea_store.rs` — is correct, but it is
wired to exactly one call site: project open (`main.rs:175`).

When a change is archived *live* (via `ds archive` while duckboard is running), the file
watcher fires and `reload_and_reconcile` reloads project data and migrates chat
subscriptions — but it never re-runs idea reconciliation. The linked idea stays stuck in
`Change` until the next restart. Both the `ViaChange` and `Orphaned` sub-kinds are
affected, and the whole path has zero behavioral test coverage, which is why it regressed
silently.

## Scope

```
caps/
├── archive/  audit/  merge/  parse/   (unchanged)
└── ideas/                ← NEW area (first duckboard capability)
    └── reconcile/        ← NEW
        └── spec.md
```

### New capabilities

- `ideas/reconcile` — the idea-drift contract. A change-state idea whose linked change was
  archived moves to `Archive`/`ViaChange`; whose change directory vanished moves to
  `Archive`/`Orphaned`; an already-archived idea is left untouched. The pass runs both at
  project open and live when a change is archived mid-session, and reports each move (old
  path -> new path) so the list selection and any open pinned tab tracking that idea
  follow it to the archive.

### Out of scope

- Re-scanning the ideas list from disk during a session — reconcile operates on the
  in-memory list; keeping that list fresh against external idea edits (which would also
  require watching the ideas data directory) is a separate change.

- The `Manual` archive sub-kind, which is user-driven and already correct.

- The `ds archive` CLI, which has no knowledge of ideas and stays that way.

- Any in-app "archive change" button — archival continues to happen externally and be
  detected through the file watcher.

## Impact

duckboard only — duckpond and `ds` are untouched.

- `reconcile` gains a return value: the set of moves it performed.

- `reload_and_reconcile` gains a reconcile step plus the UI-follow that re-points the
  selection and pinned tab for each moved idea.

- No data migration. The idea file format and the `ArchiveKind` enum are unchanged.
