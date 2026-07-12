# User choice transcript

Show mid-turn structured questions as chips and keep a clear question→answer pair in the
session log after the user responds.

## Motivation

When an agent parks a structured question, duckboard only surfaces option chips. The
question text already arrives on the wire but is discarded, so users often answer without
seeing what was asked—especially when the model wrote little prose. After a pick or
freeform reply, nothing durable in the session records that exchange, so reopening the
chat loses the Q→A trail.

Why now: the choice shell and in-band answer path are already live; the missing piece is
product-facing display and host-side history, not harness plumbing.

## Intent

- While a mid-turn choice is pending, the question text appears as a full-width chip above
  the option chips

- The question chip uses the default chat background (reads like agent content); option
  chips keep the existing quiet-accent treatment

- Choosing an option or submitting freeform text completes the choice in-band on the agent
  wire (not as a new user turn)

- On settle, the session stores two blocks: the question chip content and the user’s
  answer chip content (selected label or freeform text)

- Settled answer chips omit hotkey prefixes; freeform answers use the same answer-chip
  presentation as a pick

- On cancel, nothing from that question is kept in the session log

- Reloaded sessions render the stored pair with the same chip language as the live prompt

## Non-goals

- Multi-question questionnaires beyond the existing first-question-only surface
- Changing oneshot reply-hint chips or empty-composer defaults
- Showing unselected options in history after settle
- Changing harness wire formats or permission-tool protocols
- Auto-recovering question text when the agent never sent a prompt
