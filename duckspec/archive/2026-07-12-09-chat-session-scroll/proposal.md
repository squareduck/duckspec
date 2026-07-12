# Chat session scroll

When a chat session is first opened or the active session changes, show the latest
messages; when only the area changes, keep that session’s remembered viewport.

## Motivation

Selecting an idea under Exploration, Change, or Archive (and the same pattern in Change’s
list and chat session tabs) swaps the chat content while the viewport stays at the top of
the log. Users expect the latest turn when they open or switch sessions. Area switches
already try to restore remembered position after the shared scrollable rebuilds; session
identity changes do not get a deliberate “show latest,” and layout scroll-preservation can
replay the previous session’s offset onto the new one.

Why now: long exploration and change chats make landing at the top the default experience
of bouncing between ideas; the restore vs snap policy is already clear enough to record
without further discovery.

## Intent

- Opening a chat session for the first time (first time that session becomes the visible
  chat) shows the latest content — viewport at the bottom, stick-to-bottom engaged

- Changing the active session in the chat panel (session tab select or new session) also
  shows latest the same way

- Scope picks that surface a different chat (idea or change list selection) follow the
  same rule: the newly active session opens at latest

- Navigating between areas alone does not force latest; it restores that session’s
  remembered viewport (stuck-to-bottom or last offset)

- Scroll-preservation for layout noise does not cross session identity changes — it only
  keeps position when the active session did not change

## Non-goals

- Changing streaming stick-to-bottom or mid-turn auto-scroll while reading history
- Persisting scroll offset across app restarts
- Redesigning chat landmarks, find-in-chat, or answer-jump behavior
- New session UX beyond scroll when the active session changes
