# @ Chat transcript

## ~ Live vs settled

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

## @ Segment model

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
