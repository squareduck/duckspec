# Wrap-safe default prompt previews

Composer reply suggestions soft-wrap with row height that follows content so long lines
stay fully readable and never overlap the next row. The oneshot is soft-guided to keep
each REPLY ≤ 100 characters; the full string is never hard-truncated for display or send.

## Motivation

Long reply suggestions already wrap in the empty-input defaults list, but each row is
locked to a single line height. Wrapped text paints through the next suggestion, so
defaults are hard to scan and easy to mis-send. Soft layout that owns its height is the
hard guarantee. A 100-character instruction hint reduces how often multi-line rows appear
without fighting the model with silent cuts.

## Scope

```
caps/
└── chat/
    └── default-prompts/   ← MODIFIED (layout + oneshot length hint)
        ├── spec.md
        └── doc.md
```

### New capabilities

- none

### Modified capabilities

- `chat/default-prompts` — defaults list rows soft-wrap within the composer width, grow
  vertically so consecutive rows do not overlap, and keep the full suggestion text
  visible; oneshot instruction soft-asks each `REPLY` ≤ 100 characters; parse and send
  still keep the full string

### Out of scope

- Hard truncation or ellipsis of suggestion text (display or parse)

- Click-to-select on suggestion rows

- Changes to readiness, Tab/Enter, effective-list construction, or the lifecycle heuristic
  path

- `chat/obvious-bubble` and other composer chrome

- Cap count (still ≤ 3)

## Impact

```
duckchat                         duckboard
┌─────────────────────┐          ┌──────────────────────────┐
│ REPLY_SUGGEST_      │ soft N   │ view_default_prompt_list │
│ INSTRUCTION (+≤100) │─────────▶│ soft-wrap + grow rows    │
│ parse: still full   │  full    │ no Fixed LINE_HEIGHT     │
└─────────────────────┘  text    └──────────────────────────┘
```

- Small and self-contained: instruction string plus list view layout; no API break

- Spec delta for presentation and instruction wording; instruction phrase is easy
  `test:code`; row non-overlap is primarily visual
