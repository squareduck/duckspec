# Resync reminder on next send

Carry a captured unsynced draft into the next outgoing prompt as an appended
system-reminder, then clear it.

## Prerequisites

- [x] @step unsynced-draft-capture-and-durability

## Context

The reminder must ride the message channel appended **after** the user's text:
`system_additions` only takes effect on a session's first turn, and front-inlining breaks
slash-command parsing (see the priming comment in `send_prompt_text`). Wording frames the
draft as the agent's own already-sent reply that was interrupted before its runtime
recorded it, and says not to respond to the block itself.

## Tasks

- [x] 1. In `send_prompt_text` (`crates/duckboard/src/area/interaction.rs`), when
         `unsynced_draft` is set, append a `<system-reminder>` block holding the draft
         after the user's text in the outgoing prompt and clear the field; the transcript
         message keeps only the user's text

- [x] 2. Persist the session after clearing so a resend cannot replay a stale reminder

- [x] 3. @spec chat/cancel-resync Resync reminder on next send: The next send carries the draft after the user's text

- [x] 4. @spec chat/cancel-resync Resync reminder on next send: The reminder rides only one send
