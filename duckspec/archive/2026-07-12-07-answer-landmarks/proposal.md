# Answer landmarks

Make the latest agent Answer easy to spot with a full-width contrast band, and add
keyboard jumps between Answer starts and chat history ends so long transcripts stay
navigable.

## Motivation

In long chats, assistant Answers flow as plain prose on the same background as everything
else. After a long reply — or when scrolling through older turns — it is hard to see where
new reading should start. User prompts already read as distinct cards; agent Answers do
not. Why now: transcript volume is already high in daily use, and the calm Answer
presentation is settled enough that a light landmark (visual + shortcuts) can land without
reworking the whole chat chrome.

## Intent

- The **most recent Answer** has a **full-width**, slightly higher-contrast background
  across the chat column — a band, not a card or bubble

- That tint always tracks the **latest** Answer (including while it is still streaming
  once Answer text exists); older Answers stay on the normal chat surface

- **⌘↑** / **⌘↓** jump to the start and end of chat history

- **⌘←** / **⌘→** jump to the start of the previous / next **Answer** block (Answer tops
  only; Thinking and Activity are not anchors)

- At the first or last Answer, further prev/next is a no-op (no wrap)

- Shortcuts work when the composer is focused (⌘-arrows move the transcript; bare arrows
  stay in the input); open modals still own their own key handling

- Navigation and highlight are independent: jumps may land on any historical Answer; the
  contrast band stays on the latest only

## Non-goals

- Card/bubble chrome or left-rail-only markers for Answers
- Tinting Thinking, Activity, or the full last exchange (user + agent)
- Redesigning user message cards or the overall calm transcript model
- Wrapping reply navigation or new find/search features beyond these jumps
- Configurable keybindings or a settings UI for this behavior
