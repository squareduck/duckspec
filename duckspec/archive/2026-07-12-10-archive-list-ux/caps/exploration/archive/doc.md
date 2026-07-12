# Exploration archive

Duckboard soft-archives explorations in per-project data so finished brainstorms leave the
live list without wiping chats until the user hard-removes them.

## Live vs archived

An exploration is **live** when it has no archive stamp and **archived** when it does. The
stamp is an archive-time value stored with the exploration record in duckboard-only data
(`explorations.json`). It is not a `duckspec/archive/` folder and is not produced by
`ds archive`.

```
| State | Live lists (Change picker, Dashboard Explorations) | Chats |
| --- | --- | --- |
| Live | Shown if not idea-owned | Present |
| Archived | Hidden from live lists | Kept until remove |
| Removed | Gone | Deleted with the scope |
```

Idea-owned explorations stay on the Ideas surface and are not shown on Change/Dashboard
exploration lists whether live or archived.

## Hover control

On the Change list, each exploration row has one leading control on hover. Its meaning
depends on archive state:

```
live      ──activate──► soft archive (stamp set; chats kept)
archived  ──activate──► remove (scope + chats deleted)
```

Remove keeps the existing arming rule: when the exploration has chat sessions, the first
activation arms and a second commits; when it has none, the first activation removes
immediately. Soft archive does not require arming.

## Persistence

Old exploration records without an archive field load as live. Archiving updates the
record in place; chat directories under the exploration's scope stay until remove.
