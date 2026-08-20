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
per-block editors (table-capable `TextEdit` state). Markdown highlight and oversized
user-card prefix follow `chat/oversized-messages`. That work is the expensive part when
answers are long or contain GFM tables.

## ~ Editor refresh

Materialization reuses work where content did not change:

```
block i after materialize
        │
        ├── lines unchanged vs previous  → keep existing editor
        │
        ├── live answer/thinking, suffix grow only
        │       → refresh editor in place (append lines, partial highlight)
        │
        └── kind change / new index / non-suffix edit
                → full editor rebuild for that index
```

Earlier settled messages (user turns, completed answers) therefore avoid full re-highlight
on every live-answer tick. An oversized user card whose display prefix is unchanged also
reuses. Only the growing tail pays the refresh cost; when the segment list reshapes,
affected indices rebuild while unchanged prefixes still reuse.

## ~ Relationship to other chat capabilities

```
| Capability              | Owns                                              |
| ----------------------- | ------------------------------------------------- |
| chat/transcript         | Segment list from session + pending buffers       |
| chat/stream-ui          | When that view is materialized and how editors    |
|                         | / hybrid layout are refreshed under load          |
| chat/oversized-messages | Size gate; user-card prefix and copy full;        |
|                         | composer/queue highlight skip                     |
| chat/persistence        | When the session is written to disk               |
| editor/md-table         | Pure table geometry for a line buffer             |
```

Transcript construction, persistence schedules, and oversized display policy are
independent of this capability. Materialization reads the same session the transcript
model describes; it does not change segment rules or flush intervals.
