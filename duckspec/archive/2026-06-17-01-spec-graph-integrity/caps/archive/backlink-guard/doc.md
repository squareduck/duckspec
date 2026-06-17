# Archive backlink guard

A safety check that runs while archiving a change into the main capabilities. It refuses
to finalize an archive that would leave a live `@spec` backlink pointing at a scenario the
archive removes — preventing silent spec drift at the moment it would be introduced.

A `@spec` backlink in source code names a capability, requirement, and scenario. Archiving
rewrites capability specs: scenarios can be removed, renamed, or replaced. If a backlink
points at a scenario the archive deletes, that backlink becomes a dangling reference. The
guard catches this before the archive is written, while it can still be cleanly refused.

## What counts as an orphan

The guard answers one question: *would this archive break a backlink that works today?* It
is the difference between two resolutions, not an absolute check.

```text
backlink resolves now?   resolves after archive?   verdict
──────────────────────   ───────────────────────   ───────────────────
yes                      yes                        fine
yes                      no                         ORPHAN (archive caused it)
no                       no                         pre-existing, not flagged
no                       yes                        archive fixes it, fine
```

Only the second row is an orphan. A backlink that was already broken before the archive is
the audit's concern, not the guard's — the guard blames the archive only for breakage the
archive introduces.

Only capability spec changes move the needle. Doc changes never add or clear an orphan,
because backlinks resolve against specs, not docs.

## Where it runs

The guard runs after the archive plan is computed but before anything is written, so a
refusal costs nothing.

```text
compute plan ──→ project post-archive specs ──→ guard
                                                  │
                            no orphans ───────────┼──→ write caps, move to archive
                                                  │
                            orphans, no override ─┴──→ abort, nothing written
```

Because it evaluates the same projected spec content the archive would persist, the check
and the write never disagree.

## Refusal and override

```
| Outcome | Condition | Effect |
| --- | --- | --- |
| Refuse | orphans detected, no `--allow-orphans` | command fails, names the offending files, working tree untouched |
| Warn | orphans detected, `--allow-orphans` | warning naming the files, archive proceeds |
| Proceed | no orphans | archive proceeds silently |
```

Refusal is the default because an orphaned backlink is silent drift — code that claims to
verify a scenario that no longer exists. The `--allow-orphans` escape hatch exists for the
cases where the drift is intentional and will be cleaned up separately; it downgrades the
stop to a visible warning rather than hiding it. The offending files are always named so
they can be updated or removed.
