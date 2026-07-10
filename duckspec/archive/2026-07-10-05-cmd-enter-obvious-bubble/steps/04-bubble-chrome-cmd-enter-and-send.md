# Bubble chrome, cmd-enter, and send

Ghost user bubble in the chat view, ⌘↩ / click activation, and send via the normal user
message path without storing chrome in the transcript.

## Prerequisites

- [x] @step obvious-bubble-pure-helpers

## Tasks

- [x] 1. In `widget/agent_chat.rs`, render a greyed faux user bubble (send text + ⌘↩ hint)
         when `bubble_visible` holds; place it after real transcript content and above the
         composer; add `Msg::SendObvious` (or equivalent) on activation

- [x] 2. In `area/interaction.rs`, handle activation: when visible, call
         `send_prompt_text` with `bubble_send_text` only — never the oneshot list active
         entry

- [x] 3. Bind ⌘↩ (chat-focused) to the same activation path only when the bubble is
         visible; do not change empty Enter (list-only) semantics

- [x] 4. @spec chat/obvious-bubble Activation send: Activation sends lifecycle text when visible

- [x] 5. @spec chat/obvious-bubble Activation send: Activation is a no-op when not visible

- [x] 6. @spec chat/obvious-bubble Activation send: Send text ignores oneshot list when both differ

- [x] 7. @spec chat/obvious-bubble Ephemeral chrome: Visible bubble is not a stored user message
