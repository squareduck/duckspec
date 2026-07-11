# Fast response and structured questions

Give agents a real mid-turn choice path in duckboard by reusing the empty option-chip
shell under a neutral “fast response” name, wired for Claude and Grok through the shared
ACP host.

## Motivation

Agents already have structured question tools, but the host cannot answer them: mid-turn
agent→client requests are auto-null’d so headless turns never deadlock, Claude’s question
tool is disallowed, and Grok’s is disabled at launch. Users fall back to freeform chat or
the turn fails when a question slips through.

A generic option-chip shell (⌘-number options, ⌘⌫ cancel) already exists from the retired
lifecycle “auto message” path, but product code leaves it empty and the naming still says
“obvious chrome,” which means lifecycle next-step rather than multi-source quick picks.
That shell was deliberately left ready for structured questions; next-card ghost/Tab now
owns post-turn handoffs, so mid-turn questions are the missing channel.

Why now: both harnesses run through ACP, so one host choice loop can serve multi-harness
questions instead of two ad-hoc UIs, and the rename should land with the first real
population source so the shell is not still named for a product path it no longer
implements.

## Intent

- Mid-turn, when an agent needs a structured user choice, the chat shows ordered
  fast-response chips the user can activate with ⌘1…⌘n

- ⌘⌫ is a dedicated cancel action for that choice (not a normal composer edit)

- Answering completes the agent’s request in-band and the same turn continues; it is not
  faked as a new user message

- Claude and Grok both get this path via the shared ACP client, with harness-only
  enablement and translation at the edges

- Tool execution stays auto-approved; this change is about questions / structured choices,
  not interactive tool-permission prompts

- The option shell is renamed and described as **fast response** — source-neutral so later
  work (e.g. oneshot hints) can fill the same chips without another rename

- While waiting for a choice, chrome is allowed to show even though a turn is open
  (awaiting-user is distinct from “busy streaming”)

## Non-goals

- Populating fast-response chips from oneshot reply hints or other non-question sources
  (same shell later, not this change)

- Interactive tool-permission UI (allow/deny tool runs stay auto-approved as today)

- Full multi-select, freeform “Other,” or multi-question interview parity beyond what
  sequential single-select chips reasonably cover in v1

- Changing next-card / ghost / Tab composer authority or meta-card syntax

- Redesigning the composer footer or transcript tool-card presentation beyond what
  answering questions requires
