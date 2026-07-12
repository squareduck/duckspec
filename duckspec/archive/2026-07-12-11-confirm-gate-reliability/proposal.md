# Confirm gate reliability

Stop the grok confirm-gate loop by fixing both of its legs: cancelled turns silently
vanishing from the agent's own history, and the `/ds-spec` turn shape that invites
answer-rewriting on pure-text gate turns.

## Motivation

On `/ds-spec`, grok routinely re-presents the first capability outline for several
`confirm`s in a row. The `archive-list-ux` chat log shows why: grok rewrites the outline
reply mid-turn, duckboard's thrash budget trips and cancels the turn while keeping the
last draft locally — but the cancelled turn never enters grok's own session history. The
user then confirms a reply the agent doesn't know it sent, so the agent answers the last
gate *it* can see: the map. Every cancellation (thrash trip or user cancel) creates the
same silent transcript divergence; `/ds-spec` is just the only flow with chained identical
`confirm` gates and pure-text gate turns, so it's where the divergence becomes a loop.

Why now: the split-turn workaround added to the spec template made the loop more frequent
and the UX worse, and has been reverted — leaving no mitigation in place for a flow that
runs in nearly every change.

## Intent

- When a turn is cancelled after the user-visible reply was kept, the agent learns what
  the user saw before it processes the next message — duckboard's transcript is the single
  source of truth, for every harness

- A `confirm` is always answered against the gate the user actually confirmed, even
  directly after a cancelled or thrash-stopped turn

- The spec workflow presents the capability map together with the first capability's
  outline, so every subsequent `confirm` lands on a turn that starts with file writes
  rather than a pure-text gate reply

- Confirming through an entire spec pass involves no repeated or re-presented gates under
  normal operation

## Non-goals

- Fixing grok-4.5's answer-rewriting itself, or depending on upstream grok CLI changes

- Writing into any harness's private session storage

- Removing or loosening the answer-thrash budget

- Per-gate distinct confirm tokens (`confirm map`, `confirm <path>`) — kept as a future
  fallback

- Changing meta-card syntax or gate chrome in other templates
