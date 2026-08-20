# Chat oversized messages - Design

Oversized user transcript bodies render as an unhighlighted source prefix with a copy-full
footer. Composer and queue skip highlight at the same gate but keep the full buffer.
Answer, Thinking, and Activity are unchanged.

## Size helper

One helper next to chat highlight/rebuild (`crates/duckboard/src/area/interaction.rs`),
used as predicate, prefix, and highlight gate:

```
MAX_LINES = 200
MAX_CHARS = 16_384
```

Source characters are `lines.join("\n").chars().count()`.

```
is_oversized(lines) -> bool
display_prefix(lines) -> Vec<String>   // exact source prefix; no ellipsis line
maybe_highlight(editor, highlighter, skip: bool)
  skip || is_oversized(editor.lines) → highlight_spans = None
  else markdown highlight_lines
```

`is_oversized` is true when line count exceeds `MAX_LINES` or source characters exceed
`MAX_CHARS`. `display_prefix` walks lines until the next line would exceed either cap; a
single over-budget line is sliced by characters.

## User cards

Truncate when turning a User segment into a display `Block`, not inside
`build_transcript_segments`. Session and segment lines stay full.

If the **full** user body is oversized:

- `Block.lines` = `display_prefix`
- `Block.truncated = true` (new field, default false)
- `maybe_highlight(..., skip=true)` — the prefix is under the gate, so skip is explicit
- Footer inside the user card, under the editor: `shown of total characters · Copy full`

`Msg::CopyFull(idx)` joins the **current** matching user segment body from the session and
`clipboard::write`s it. Do not cache the full string on the block (avoids a 2MB join on
every stream tick). Ordinary editor copy still copies the prefix.

Later rebuilds see a stable prefix → existing `Reuse`. Find matches only in the prefix.

Non-user blocks are not truncated and keep today’s highlight path.

## Composer and queue

No truncate, no footer. Composer `max_rows` stays 20.

`rehighlight_input` and `make_queue_editor` call `maybe_highlight(..., skip=false)` so the
oversized predicate on the **full** buffer drives skip. Dropping back under the gate
restores highlight.

## Call sites

```
| Surface | When | Behavior |
| --- | --- | --- |
| User card | `rebuild_chat_editor` | Prefix + `skip=true` when full body is oversized |
| Composer | Input mutation and `rehighlight_all` | Skip when the input buffer is oversized |
| Queue | `make_queue_editor` and `rehighlight_all` | Same skip as composer |
```

`rehighlight_all` today syntects every chat editor and the composer on the UI thread, and
it does not touch `queue_editor`. It zips `chat_blocks` / `chat_editors` and skips when
`truncated`; composer and queue go through `maybe_highlight`.

## Compatibility

Display-only. No session JSON change. Existing sessions with a huge user paste get the
prefix on next materialize.

## Rejected here

- Truncating or skip-highlighting Answer / Thinking (agent echo of the JSON can still
  hitch)

- Expand-in-place

- Truncating composer or queue

- Transcript virtualization and async highlight for ordinary-sized messages
