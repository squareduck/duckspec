# Chat stream UI

How duckboard keeps the live chat transcript responsive while a turn streams: session text
always lands immediately, but the editors the UI paints are rebuilt on a bounded cadence
and on structural events—with settled blocks reused and hybrid table layout shared across
layout passes.

## Apply vs materialize

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

## When materialization runs

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

## Editor refresh

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
on every live-answer tick. Only the growing tail pays the refresh cost; when the segment
list reshapes, affected indices rebuild while unchanged prefixes still reuse.

## Hybrid layout cache

Chat blocks enable hybrid layout (`md_tables` + wrap): prose rows plus `editor/md-table`
regions for complete GFM tables. Layout geometry is keyed by pane width, wrap flag, and
line-buffer identity/version.

```
layout / update / draw
        │
        ▼
  hybrid cache
        │
        ├── same key → share cached geometry (no table re-scan, no deep tree copy)
        │
        └── key change → recompute EditorLayout (calls table layout as needed)
```

Settled table-heavy messages stay cheap across frames: the cache hit shares one layout
tree. The live answer still recomputes when its buffer version changes, but only as often
as materialization runs—not once per stream token and not with a full layout-tree clone on
every paint of unchanged blocks.

## Relationship to other chat capabilities

```
| Capability        | Owns                                              |
| ----------------- | ------------------------------------------------- |
| chat/transcript   | Segment list from session + pending buffers       |
| chat/stream-ui    | When that view is materialized and how editors    |
|                   | / hybrid layout are refreshed under load          |
| chat/persistence  | When the session is written to disk               |
| editor/md-table   | Pure table geometry for a line buffer             |
```

Transcript construction and persistence schedules are independent of this capability.
Materialization reads the same session the transcript model describes; it does not change
segment rules or flush intervals.

## Answer draft and thrash budget

While streaming, the open answer is a **live draft** in the pending answer buffer:

```
answer draft ──reasoning──► draft stays uncommitted
             ──answer───► draft replaced (prior body discarded)
             ──tool──────► draft committed, then tool row
             ──turn end──► draft committed
```

Each answer-after-thought replacement increments a per-turn counter against a small fixed
client budget (implementation constant). Replacements within budget keep streaming; the
first replacement that would exceed the budget cancels the turn, keeps the last draft, and
shows a short stop notice.

```
within budget  →  keep streaming (draft replaced)
over budget    →  cancel turn · keep last draft · short stop notice
```

Tool use resets the counter so a new answer span after tools starts fresh. The stop notice
is not a second assistant write-gate; the last draft remains the turn’s answer.
