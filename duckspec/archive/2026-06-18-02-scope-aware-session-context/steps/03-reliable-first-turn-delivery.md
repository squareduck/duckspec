# Reliable first-turn delivery

Move the scope orientation off the flaky `--append-system-prompt` channel onto the
first-turn priming body, and prime even when no `AGENTS.md` exists.

## Prerequisites

- [ ] @step enriched-scope-hook

## Context

In `send_prompt_text` (`area/interaction.rs`) the new-session branch
(`claude_session_id.is_none() && messages.is_empty()`) currently gates the priming turn on
`AgentsMarkdownHook` returning `Some`, and the scope blurb + `PATH_REFERENCE_NOTE` ride
`system_additions` (which the CLI drops). Generalize: assemble a combined priming body
from the AGENTS.md text (if any), the `CurrentScopeHook` blurb, and `PATH_REFERENCE_NOTE`,
and prime whenever that body is non-empty. The follow-up dispatch
(`pending_followup_prompt`, selection attachments, idea-description injection) and the
legacy no-session-id fallback path are unchanged.

## Tasks

- [x] 1. Build the priming body from the available orientation parts (AGENTS.md text if
         present, the `CurrentScopeHook` blurb, `PATH_REFERENCE_NOTE`), joined with the
         existing single-dot-ack instruction

- [x] 2. Trigger the priming turn whenever the assembled body is non-empty, rather than
         only when `AGENTS.md` is present

- [x] 3. Leave `system_additions` empty on the priming `TurnRequest`, so all orientation
         rides the message body

- [x] 4. @spec session/scope Reliable first-turn delivery: The first turn's message body carries the scope orientation

- [x] 5. @spec session/scope Reliable first-turn delivery: Orientation is present when the project has no AGENTS.md

- [x] 6. @spec session/scope Reliable first-turn delivery: A resumed session does not repeat the orientation
