# Chat stream UI stays responsive with tables

Keep duckboard’s chat usable while a turn streams—especially when answers include GFM
tables—by bounding live transcript rebuild work and avoiding table-layout thrash on the UI
thread.

## Motivation

After GFM tables started rendering in chat `TextEdit`, mid-turn appends often make scroll
and the composer hitch or hang for seconds. Streaming plus scroll or type is the normal
path, and hybrid table layout joined every chat block on that path.

Why now: the regression is daily-use pain on the core agent loop, and the pure table
kernel (`editor/md-table`) is already correct—this is view-layer cost around live
rebuilds, not a layout-geometry bug.

## Scope

```
caps/
├── chat/
│   ├── transcript/     (unchanged — segment model only)
│   ├── persistence/    (unchanged)
│   └── stream-ui/      ← NEW
│       └── spec.md     (live rebuild budget / reuse / responsiveness)
└── editor/
    └── md-table/       (unchanged — pure GFM layout kernel)
```

### New capabilities

- `chat/stream-ui` — Rules for the live chat UI while a turn is in progress: how often the
  transcript is rebuilt from stream events; that settled blocks keep their editors when
  content is unchanged; and that hybrid table layout for chat must not force full
  recompute or full layout-copy work for every stream delta and every layout pass on
  settled content.

### Modified capabilities

- None (reuse `editor/md-table` and `chat/transcript` as-is)

### Out of scope

- Full chat-list virtualization (long-history scroll architecture)

- Changes to GFM table recognition or pure layout geometry contracts in `editor/md-table`

- Harness/ACP stream protocol or persistence flush interval

- Non-chat editors (file tabs) performance, except shared hybrid-layout helpers if they
  land as an implementation detail of this change

## Impact

```
AgentEvent (ContentDelta…)
        │
        ▼
  stream-ui policy  ← NEW contract
        │
        ▼
 rebuild_chat_editor / chat TextEdits
        │
        ├── settled blocks: reuse (no re-highlight)
        └── live answer: bounded hybrid md_table layout
                │
                ▼
        composer stays on same UI thread (less freeze)
```

- **duckboard only** — stream event → rebuild path (`interaction` / `main`), chat
  `TextEdit` (`md_tables` + `fit_content`), hybrid layout cache in `text_edit/render`.

- No duckpond / `ds` schema changes expected.

- Observable behavior: fewer freezes mid-stream; table answers may update in small batches
  rather than every token; visual correctness of settled tables unchanged.
