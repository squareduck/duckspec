# @ Chat stream UI

## = Answer draft and thrash budget

Answer draft

## ~ Apply vs materialize

Stream handling has two layers:

```
agent event
    │
    ▼
 session apply          always — pending buffers / messages / turn flags
    │
    ▼
 chat UI materialize    gated — builds chat_blocks + editors for paint
```

**Apply** updates the session (pending answer/reasoning text, tool rows, and turn flags).
Nothing needed for a correct transcript is dropped because the UI is deferred.

**Materialize** rebuilds the view inputs the chat column paints: transcript blocks and
per-block editors (table-capable `TextEdit` state). Markdown highlight and oversized
user-card prefix follow `chat/oversized-messages`. That work is the expensive part when
answers are long or contain GFM tables.

## ~ Answer draft

While streaming, the open answer is a **live draft** in the pending answer buffer:

```
answer draft ──reasoning──► draft stays uncommitted
             ──answer───► draft replaced (prior body discarded)
             ──tool──────► draft committed, then tool row
             ──turn end──► draft committed
```
