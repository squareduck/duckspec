# Chat oversized messages

Oversized user chat text stays cheap to show: user cards render an unhighlighted source
prefix with a way to copy the stored full body, and the composer and queue skip highlight
without truncating.

## Size gate

A body is oversized when it exceeds the **oversized line cap** or the **oversized
character cap**. Character count is source text: the characters of the lines joined with
newlines, matching the stored message body.

```
            line count > line cap
                       OR
     source characters > character cap
                       │
                       ▼
                   oversized
```

The prefix of an oversized body is an exact source prefix: whole lines until the next line
would exceed either cap, then a character slice of that line if needed. Nothing extra is
inserted into the prefix.

## User cards

Transcript segments and the session keep the full user body. Only the display block for a
User segment is gated.

```
session user body (full)
        │
        ▼
  oversized? ──no──► full card, markdown highlight
        │
       yes
        ▼
  display prefix, no highlight
  copy full → current session body
```

Copy full reads the matching user segment from the session at click time, not a snapshot
taken when the card was first truncated. Ordinary editor copy still copies whatever is in
the display prefix.

Find-in-chat matches the displayed prefix only.

## Composer and queue

Composer and queue keep their full buffers and are not truncated. When a buffer is
oversized, syntax highlight is skipped; when it drops back under the gate, highlight
returns. Theme rehighlight uses the same skip.

## Ownership

```
| Capability              | Owns                                              |
| ----------------------- | ------------------------------------------------- |
| chat/transcript         | Segment list from session; User/Answer/Thinking   |
|                         | bodies stay full                                  |
| chat/stream-ui          | When editors materialize and how they refresh     |
| chat/oversized-messages | Size gate; user-card prefix and copy full;        |
|                         | composer/queue highlight skip                     |
```

Answer, Thinking, and Activity presentation are unchanged. Activity tool dumps keep their
own short output cap.
