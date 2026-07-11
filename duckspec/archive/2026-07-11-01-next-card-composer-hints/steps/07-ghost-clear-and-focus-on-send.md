# Ghost clear and focus on send

Hide next-action ghost while streaming; keep chat input focused after empty Enter (stable
row + refocus).

## Prerequisites

- [x] @step next-action-composer
- [x] @step calm-tint-and-tab-marker-placement

## Context

From followup `reviews/02-followup-ghost-and-focus-on-send.md`: ghost ignores streaming
while Tab marker and empty Enter already disarm; multi-next input row unwraps when
streaming starts and drops iced focus.

## Tasks

- [x] 1. Gate next-action ghost on not streaming (update `next_ghost_text` and/or
         `agent_chat` placeholder so ghost clears as soon as a turn is in progress)

- [x] 2. Keep a stable input-row widget structure whether or not the tab-available marker
         is visible (avoid bare TextEdit vs row shape change on stream start)

- [x] 3. Refocus the chat input after empty-next / `SendPressed` send (parity with Tab
         cycle `focus_chat_input`)
