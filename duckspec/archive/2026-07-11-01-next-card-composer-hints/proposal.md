# Next-card composer hints

Align empty-composer next actions with trailing agent `next` meta cards so the UI and
templates share one authority for “what you can do next,” and relegate optional model
reply suggestions to a single under-input affordance.

## Motivation

Templates already emit ranked `next` meta cards for handoffs and write gates. Duckboard
still offers a separate disk-phase chip ladder (auto messages) and optional under-input
suggestions that also drive empty Enter, so empty-composer authority is split and users
see two systems for the same job.

Why now: meta-card syntax is stable across templates; aligning the composer before more
stages depend on chips avoids dual brains becoming load-bearing. Write-gate and handoff
tokens (`confirm`, `reject`, slash commands) should be the same strings the agent already
emits, not UI-invented labels.

## Intent

- After the first turn, empty-composer next actions come only from a trailing `next` meta
  card on the last assistant message (ranked order, capped as for meta cards).

- The active next action appears as ghost text in the empty input; empty Enter sends it;
  Tab / Shift-Tab cycle when there is more than one; a small tab-available marker shows
  when cycling is possible — no full next-action list under the input.

- No trailing `next` after the first turn means no next-action ghost or cycle (missing
  offers are a template or agent fix, not a UI fallback from disk phase).

- Empty sessions (zero turns) may still seed one bootstrap action from lifecycle so a
  brand-new chat is not dead; that path ends as soon as there is at least one turn.

- Optional agent reply suggestions (settings-gated) produce at most one freeform prompt
  under the input — a natural next user response from the last user and assistant
  messages, not lifecycle autocomplete. That prompt is sent only via Shift-Enter when the
  input is empty; when none is available, Shift-Enter does nothing for this purpose. Empty
  Enter never sends the oneshot suggestion.

- Lifecycle / affirm / decline auto-message chips are no longer how next steps or write
  gates are offered. Their chrome core stays in place, stripped of phase fluff, so a later
  change can wire structured questions (e.g. ⌘-number options, ⌘⌫ cancel) without
  rebuilding the shell. The auto-messages setting is removed entirely.

- Transcript lines that belong to meta cards (`write` / `next`) use a distinct quiet
  background so gates and handoffs are scannable in history.

## Non-goals

- End-to-end structured question-tool support (harness protocol, turn pause, answer
  routing) — only leave option-chrome core ready to wire later.

- Changing meta-card syntax or inventing new agent-facing card kinds.

- Replacing or redesigning the composer footer (model picker, context usage, resend hint).

- Auto-fixing missing `next` cards from disk phase after the first turn.

- Broad transcript restyling beyond meta-card differentiation.
