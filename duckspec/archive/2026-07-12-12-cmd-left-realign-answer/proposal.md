# Cmd-left realign to current answer

When the chat viewport sits below the top of the current Answer, first previous-answer
jump re-aligns to that top so a tall latest reply is readable before stepping older.

## Motivation

At the bottom of a long transcript, previous-answer navigation treats the latest Answer as
already “current” and immediately steps to the one before it. If that latest agent reply
is taller than the viewport, the start of the message the user wants to read is skipped
entirely.

Why now: answer landmark jumps are already the way back into long chats; this first-press
mis-step is the common failure mode when re-entering a huge latest reply.

## Intent

- Previous-answer navigation re-aligns the viewport to the top of the current Answer when
  the scroll offset is below that top (within existing top-alignment slack)

- Only when the viewport is already at the current Answer’s top does previous step to the
  prior Answer (or no-op at the first)

- Next-answer navigation stays adjacent-step only — no re-align-first rule

- Stick-to-bottom and mid-message scroll share the same “below current top → re-align
  first” rule

## Non-goals

- Changing history top/bottom shortcuts or which segments count as Answer anchors
- Symmetric re-align behavior for next-answer jumps
- New landmark shortcuts or alternate navigation modes
- Visual highlight or band behavior for the current Answer
