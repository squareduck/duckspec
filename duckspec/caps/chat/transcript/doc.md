# Chat transcript

How the chat UI turns neutral agent events and stored session content into a calm
transcript of Thinking, Activity, and Answer segments — so the reply stays primary and
supporting work stays quiet.

## Segment model

A turn is presented as an ordered list of segments, not one bubble per event. Contiguous
assistant content of the same kind coalesces; a kind switch opens a new segment.

```
stream / stored blocks          segments
──────────────────────          ────────
Reasoning*                  →   Thinking
(ToolUse | ToolResult)*     →   Activity  (one group, N rows)
Text*                       →   Answer
```

User and system messages remain their own segments. While a turn streams, pending
reasoning and pending answer text feed live Thinking and Answer segments until they are
flushed into committed content blocks. When thought runs while an answer draft is still
open (not yet committed), the live transcript shows **one** Thinking segment and **one**
Answer segment for that draft — the answer body may rewrite in place; it does not stack
extra Answer bubbles for the same draft.

```
| Segment  | Source                         | Role in the UI        |
| -------- | ------------------------------ | --------------------- |
| Thinking | Reasoning content + pending    | Why (secondary)       |
| Activity | Tool use/result runs           | What it did (tertiary)|
| Answer   | Text content + pending         | Reply (primary)       |
```

## Activity groups

Consecutive tools form one Activity segment. Inside the group, uses and results pair by
call id, not by adjacency alone — so interleaved completions still merge into one row per
call. A completed row carries a short tool summary and the result body. A result with no
matching use still becomes a done row labeled from the tool name; it is never shown only
as a bare "done" placeholder.

When the group is expanded, each tool is one quiet row (status + summary) with truncated
output under the row when present. Collapse is group-level only: there is no nested
per-tool expand state.

Collapsed, the group summarizes as a count plus sample tool names (for example
`4 tools · Read, grep, shell`).

## Collapse defaults

```
| Segment  | Live default | Settled / reload default | Auto-collapse            |
| -------- | ------------ | ------------------------ | ------------------------ |
| Thinking | expanded     | collapsed                | when Answer follows or   |
|          |              |                          | the turn completes       |
| Activity | expanded     | collapsed                | when Answer follows or   |
|          |              |                          | the turn completes       |
| Answer   | always shown | always shown             | not collapsible          |
```

If the user toggles a Thinking or Activity segment, that choice wins: later auto-collapse
does not force it shut again for that segment.

Collapsed Thinking labels use line count (`Thinking · N lines`), not duration.

## Live vs settled

```
LIVE                                      SETTLED
────                                      ──────
Thinking open (streaming body, faded)     Thinking collapsed (line count)
Activity expanded (current tool clear)    Activity collapsed (count · names)
Answer streaming as plain prose           Answer as plain prose
```

Thinking body ink is slightly more faded than Answer prose (secondary text color) so
reasoning reads as supporting work. Headers may be more muted still. Harnesses that never
emit reasoning simply never open Thinking segments; Activity and Answer behavior still
apply. Presentation is driven only from neutral session content and stream buffers — not
from harness-specific UI branches.

## Meta-card highlighting

Answer text that contains chat meta cards (`write` / `next` quote runs from chat meta-card
recognition) marks those lines with a quiet background so gates and handoffs scan
differently from ordinary reply prose. Only lines inside a recognized card range are
tinted; surrounding Answer lines stay plain. Thinking and Activity segments are unchanged.
