# Attribute new change folders to their originating session

When an agent runs `ds create change`, bind the resulting folder to that session by
capturing the command from the tool-call stream, instead of guessing the owner from
ambient selection state during reconcile.

## Motivation

With a change-area exploration and an idea exploration alive at once,
`reload_and_reconcile` (`main.rs:2863`) uses a fixed `if`/`else` precedence that hands
every new change folder to the change-area exploration — even when the idea session
created it. The idea's frontmatter never receives its `change` link, so it reads as
orphaned.

The deeper issue: every ambient signal — selection, streaming, focus, recency — is merely
*correlated* with authorship, so all of them misfire in the concurrent case. A session can
be streaming unrelated work while a different session that already finished its turn is
the true creator. The agent's tool-call stream is the one *causal* signal for who ran
`ds create change`, and reconcile currently throws it away.

## Scope

This is an internal duckboard fix. No capability specs are created or modified.

Behavior:

- On each `ds create change <arg>` tool call, record
  `slug(arg) → originating
  session scope`, captured in the `ToolUse` handler where the
  session is already known from the routing key.

- On reconcile, look up the new folder's name in the recorded bindings and route promotion
  to that exploration, choosing `promote_exploration` vs `promote_idea_exploration` by its
  `idea_path`.

- Fall back to `active_area` when no binding exists (out-of-band creation via the CLI, or
  a non-Bash creation path).

### Out of scope

- Stale-binding cleanup — dangling entries are left in place; changes are infrequent
  enough that the map never grows meaningfully.

- The internals of `promote_exploration` and `promote_idea_exploration` — reused
  unchanged.

- Non-Bash or out-of-band creation beyond the active-area fallback.

- Any tool-call permission or interception mechanism.

## Impact

duckboard-only:

- Adds a pending-binding map to `State`.

- Adds a capture hook in the `ToolUse` handler (`main.rs:1368`).

- Rewrites the promotion branch in `reload_and_reconcile` (`main.rs:2862`), replacing the
  fixed `if`/`else` precedence with binding lookup plus active-area fallback.

- Reuses the `slug/` rule for matching a command argument to its folder name.

No changes to `duckpond` or the `ds` CLI.
