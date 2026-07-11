# @ Chat stream UI

## ~ Apply vs materialize

Stream handling has two layers:

```
agent event
    │
    ▼
 session apply          always — pending buffers / messages / thrash budget
    │
    ▼
 chat UI materialize    gated — builds chat_blocks + editors for paint
```

**Apply** updates the session (pending answer/reasoning text, tool rows, turn flags, and
the answer thrash counter). Nothing needed for a correct transcript is dropped because the
UI is deferred — except that after a thrash trip, further answer/reasoning deltas for that
turn are ignored until streaming ends.

**Materialize** rebuilds the view inputs the chat column paints: transcript blocks and
per-block editors (including markdown highlight and table-capable `TextEdit` state). That
work is the expensive part when answers are long or contain GFM tables.

## ~ When materialization runs

```
| Trigger                              | Materialize?                                |
| ------------------------------------ | ------------------------------------------- |
| Answer / reasoning content only      | No — mark dirty; wait for tick              |
| Stream UI tick (dirty + stick)       | Yes — fold in accumulated pure text         |
| Stream UI tick (dirty, scrolled up)  | No — leave dirty; keep history scroll calm  |
| Re-stick to bottom while dirty       | Yes — paint deferred pure content           |
| Tool use / tool result               | Yes — immediately (even if scrolled up)     |
| Answer ↔ reasoning channel switch    | Yes — immediately (draft need not commit)   |
| Turn complete / error / exit         | Yes — immediately                           |
| Load / send / non-stream paths       | Yes — immediately                           |
```

Pure content deltas can arrive many times per second. The stream UI tick (~100 ms while
any session is streaming) drains dirtiness when the user is following the live answer
(stick-to-bottom). If they have scrolled up to read history, pure-content dirtiness stays
deferred so the chat column is not rebuilt under their scroll; returning to the bottom
paints the deferred text. Structural events change transcript shape (new Activity, open
Thinking under a live draft, final Answer) and must paint in the same turn as the session
update.

## + Answer draft and thrash budget

While streaming, the open answer is a **live draft** in the pending answer buffer:

```
answer draft ──reasoning──► draft stays uncommitted
             ──answer───► draft replaced (prior body discarded)
             ──tool──────► draft committed, then tool row
             ──turn end──► draft committed
```

Each answer-after-thought replacement increments a per-turn counter. The budget is **two**
replacements; a third trips the client:

```
replace #1  →  keep streaming
replace #2  →  keep streaming
replace #3  →  cancel turn · keep last draft · short stop notice
```

Tool use resets the counter so a new answer span after tools starts fresh. The stop notice
is not a second assistant write-gate; the last draft remains the turn’s answer.
