# Chat persistence

How chat sessions are written to disk so history is never lost — to a crash mid-write, to
a scope migration, or to an interruption partway through a turn.

## Where sessions live

Each chat session is a file under a per-scope directory, keyed by the session's id. A
scope is a change, an exploration, or one of the fixed scopes. Migrating a session between
scopes moves its file between these directories.

## Atomic writes

A session write goes to a temporary file in the destination directory and is then renamed
into place. The rename is atomic, so a reader — or a crash — never observes a half-written
file, and a failed write leaves the previous version intact rather than truncating it to
empty. The temporary file stays on the same filesystem as its target so the rename cannot
fall back to a non-atomic copy.

## Scope migration

Migrating a scope's sessions into another scope moves files individually and merges rather
than replaces. This holds both on disk and for the in-memory view.

```
source scope            target scope (already populated)
  s1, s2         ──►       t1
                           │
                           ▼
                   t1, s1, s2   (union — nothing dropped)
```

When the same session id exists in both scopes, the copy with more messages wins and the
displaced copy is set aside rather than deleted, so a wrong choice is always recoverable.
In memory, sessions fold into the existing scope's state so its live subscriptions (agent,
terminal) keep running across the migration.

```
| Migration case | Result |
| --- | --- |
| Id present only in source | Moved into target |
| Id present only in target | Left as-is |
| Id in both | Fuller copy kept; other preserved aside |
```

## In-flight turn durability

A turn streams messages incrementally. If those messages live only in memory until the
turn completes, anything that disturbs the scope's state before completion loses them. Two
rules close that window.

- **Flush before mutate.** Any operation that migrates, replaces, or drops a scope's
  in-memory state first persists that scope's sessions. An in-flight turn is therefore
  written to disk before, for example, a promotion moves the scope.

- **Eager persistence.** While a turn streams, its session is persisted periodically — not
  only at turn completion — so an abrupt termination loses at most a bounded tail of the
  turn rather than the whole turn. Writes are coalesced so a long turn does not rewrite
  the growing session file on every message.

Together with turn-boundary saves, these bound the loss window to a short tail in the
worst case and to nothing in the ordinary migrate/promote paths.
