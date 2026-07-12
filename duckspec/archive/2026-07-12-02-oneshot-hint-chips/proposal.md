# Oneshot hint chips

Move agent reply suggestions off the under-input strip onto the same ⌘-number chip chrome
as mid-turn questions, with clear priority against ghost text and the question tool, and
remove the leftover obvious-bubble surface.

## Motivation

Agent oneshot hints render as a one-off under-input row that does not match the rest of
the chat chrome. Fast-response chips already own multi-option ⌘-number activation for
structured questions, so freeform reply suggestions fight a second, weaker surface.

Lifecycle next actions already use ghost text in the composer. When both ghost and
under-input oneshot are present, two autocomplete-like affordances compete. The dead
“obvious bubble” product is gone from the UI, but the cap and bubble-named helpers still
clutter the tree.

Why now: chip chrome and question-tool priority are stable; folding oneshot into that path
before more surfaces accumulate avoids another permanent dual UI.

## Intent

- Under-input oneshot chrome (including loading and Cmd-Enter send) is removed

- When agent input hints are on (still default off — oneshot has model cost), a settled
  oneshot may offer up to three freeform reply suggestions as fast-response chips after a
  turn, only while idle (not streaming)

- Oneshot instruction asks for up to three plain `REPLY:` lines in order: most likely
  reply, alternative reply, negative/decline reply; parsing stays simple; partial results
  show as-is

- No oneshot loading chrome — chips appear only after a non-empty settle

- Oneshot chips are hidden when a next-action ghost is available so ghost and oneshot
  never compete

- A live question-tool choice always wins over oneshot chips, regardless of which arrived
  first

- Activating an oneshot chip sends that text as a normal user message; question-tool picks
  still answer in-band

- Empty Enter remains next-action/ghost only; chips use click and ⌘-number

- The obsolete obvious-bubble capability is deleted; remaining bubble/obvious names for
  lifecycle formatting and helpers are renamed to what they actually do

## Non-goals

- Turning on agent input hints by default

- Merging next-action ghost into chips or replacing trailing `next` meta cards

- Changing question-tool wire protocol or in-band answer semantics

- Redesigning chip layout, theme, or bottom-pad behavior beyond reusing the existing shell

- New oneshot roles, typed `REPLY_*` prefixes, or richer structured suggestion schemas

- Removing lifecycle scope facts, orientation blurbs, or empty-session ghost bootstrap
  (only the dead bubble product and stale names)
