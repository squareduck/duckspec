# Priming Setup collapse

Fold the synthetic first-turn AGENTS.md / orientation inject in the chat transcript so
scroll-to-top lands on the first real user message, not the long system preamble.

## Motivation

Every new session injects a priming user message (project conventions, scope orientation,
path-reference note, single-dot ack). The agent replies with `.`. That preamble is correct
for the agent but expensive for humans: scrolling to the top of the chat lands on a wall
of standing instructions instead of the first real request. Once a conversation has real
turns, the priming body is almost never the thing someone wants to re-read by default.

## Intent

- The priming inject is still stored and still visible on demand

- By default it is collapsed so the transcript top shows real user/agent work

- Clicking expands it for inspection

- After a short delay while expanded, it auto-collapses again so a casual peek does not
  leave the preamble permanently open

- Ordinary user messages are never auto-collapsed by this behavior

## Non-goals

- Changing what the priming body contains or when priming runs
- Collapsing the assistant `.` reply or ordinary user/answer prose
- Redesigning Thinking/Activity collapse policy beyond coexisting with Setup
- Hiding priming from persistence, title summarization already skips it
