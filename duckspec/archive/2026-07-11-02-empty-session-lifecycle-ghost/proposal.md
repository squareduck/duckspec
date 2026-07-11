# Empty session lifecycle ghost and promotion focus

Empty chats should still offer the obvious next lifecycle command as composer ghost text,
and creating a change from an exploration should not steal focus from the chat input.

## Motivation

After the next-card composer rework, empty sessions are supposed to bootstrap one
lifecycle next action into the ghost so a brand-new chat is usable without agent input
hints. In practice that bootstrap is missing for empty exploration chats (no `/ds-explore`
ghost), and agent input hints default off so nothing else fills the gap. Separately, when
an exploration’s agent creates a change and duckboard promotes the session into that
change, the chat input loses focus and typing requires a re-click.

Why now: both gaps sit on the path that just shipped (next-card ghosts + create-change
promotion) and show up in the first seconds of a new chat or right after the
explore→change handoff.

## Intent

- When a chat has no messages and a lifecycle next command is known for that scope, the
  empty composer shows that command as ghost text (e.g. explore → `/ds-explore`, change
  with unfinished steps → `/ds-apply`) and empty Enter sends it

- That bootstrap does not depend on the agent input hints setting; oneshot under-input
  suggestions remain separate and still gated

- Bootstrap applies only while the session transcript is empty; after any turn, next
  actions follow existing trailing-`next` rules only

- After an exploration is promoted into the change its agent just created, the chat input
  remains focused so the user can keep typing without clicking back in

## Non-goals

- Restoring multi-option lifecycle chip ladders or auto-messages under the input

- Changing oneshot reply-suggestion behavior beyond keeping it independent of
  empty-session bootstrap

- Changing promotion attribution rules (who owns a newly created change)

- Refocusing on every project reload or unrelated scope switch
