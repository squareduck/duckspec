# New session chat ghost migration

When a new empty chat session is opened for a change, keep the previous session’s
next-action ghost options so the user can continue handoffs without re-earning them from
the agent.

## Motivation

On a change with multi-session chat, the active session often already has ranked next
actions (from a trailing `next` meta card) as composer ghost text with empty Enter and
Tab. Hitting **+** or **⌘N** opens a clean transcript, but the new empty session only sees
scope lifecycle bootstrap (or nothing). The user loses agent-ranked handoffs and must run
another turn—or settle for a weaker disk-phase ghost—before continuing.

Why now: next-action ghosts and empty-session bootstrap already ship; multi-session “start
clean, keep the handoff” is the missing continuity piece.

## Intent

- Creating a new chat session for a change (session **+** and **⌘N** when they mean new
  chat) copies next-action options from the **session that was active** at that moment

- The new session stays empty of messages; inheritance applies **only while it remains
  empty**, until its first turn

- While inherited options are active, ghost text, empty Enter, and Tab behave as they did
  for those options on the donor session (selection starts at the first action)

- If the active session had no next-action options, the new empty session keeps today’s
  empty-session behavior (lifecycle bootstrap when available)

- After the new session’s first turn, normal rules apply: trailing `next` only; no
  re-inheritance and no disk-phase fallback when a `next` card is missing

- Oneshot reply chips are not part of this migration

## Non-goals

- Persisting inherited next actions across app restart (reload may fall back to empty
  bootstrap)

- Migrating or seeding oneshot / fast-response chips

- Changing how non-empty sessions derive next actions from trailing `next` cards

- Auto-filling next actions from disk phase after the first turn

- New session creation outside change multi-session chat (e.g. content-column new file,
  add idea, new window)
