# Obvious chip tones and bottom pin

Make auto-message chrome scannable by role (blue numbered options, green/red gate/enter)
and dual-show a multi-option lifecycle ⌘↩ target; pin chrome to the bottom of the history
pane when content is shorter than the viewport.

## Motivation

Numbered lifecycle chips currently share faded paper styling with user message bubbles, so
the option list does not read as a distinct category. When the first lifecycle option also
owns ⌘↩, that action appears once as a dual-purpose green chip — the numbered and enter
roles blur. On empty or short transcripts, chrome paints at the top of the scroll pane,
which fights the mental model that chips are “last messages” sitting above the composer.

## Scope

```
caps/
└── chat/
    └── obvious-bubble/   ← MODIFIED (display tones, dual enter chip, bottom pin)
```

### New capabilities

None.

### Modified capabilities

- `chat/obvious-bubble` — Numbered multi-option chips use quiet light-blue chrome (~8%
  blue tint, same strength as enter/reject). When multi-option chrome has no affirm, show
  lifecycle[0] twice: blue numbered `⌘n  /ds-…` in order, plus green `⌘↩  <Friendly>` at
  the bottom that still sends the original command. Single-option chrome stays one green
  chip. When transcript plus chrome is shorter than the chat viewport, add space above
  chrome so chips sit above the composer while remaining in the scroll column (not an
  overlay).

### Out of scope

- Chrome composition and key-resolution rules (what options exist; what ⌘↩ / ⌘⌫ / ⌘n send)
- Oneshot default-prompts and under-input input hints
- User-message card styling beyond the intended contrast with chips
- Overlay or fixed-position chrome above the composer
- Auto-messages setting behavior

## Impact

Duckboard-only UI work (`theme`, `obvious_bubble` pure helpers, `agent_chat` view). No
harness, CLI, or chat-persistence changes. Existing chip-display scenarios need a delta
for dual enter labels and tone rules.
