# Calm chat transcript

Present agent turns as a calm, harness-neutral transcript of thinking, tool activity, and
answer segments — so reasoning is isolated and collapsible, and tool spam collapses into
quiet activity groups instead of one card per call.

## Motivation

Grok turns already stream thinking and tools on distinct event channels, but duckboard
flattens thinking into answer text and renders every tool as its own card (plus orphan **✓
done** rows). Agent turns feel noisy; the answer is hard to find. Reasoning is mostly
grok-visible today, but tool density hits every harness — fix the shared transcript layer,
not a grok-only fork.

## Scope

```
caps/
├── chat/
│   ├── persistence/     (modified — Reasoning block + load compat)
│   └── transcript/      ← NEW — segment model + calm UX
└── harness/
    └── grok/            (unchanged — already emits distinct channels)
```

### New capabilities

- `chat/transcript` — build the live/settled transcript from neutral agent events:
  contiguous **Thinking** / **Activity** / **Answer** segments; auto-collapse thinking
  when answer arrives; group consecutive tools into one activity card with quiet rows,
  id-based pairing (no **✓ done** orphans), live current-tool emphasis, settled one-line
  summaries

### Modified capabilities

- `chat/persistence` — persist `Reasoning` content blocks; sessions without them still
  load

### Out of scope

- Harness-specific render forks (no grok-only UI path)
- Changing grok/Claude event translation (already adequate)
- Activity sidebar / “hide tools forever” mode
- Redesigning the streaming-indicator chrome beyond segment interaction
- Claude reasoning channel (no-op if events never arrive)

## Impact

```
duckchat events (neutral)     duckboard session          view
─────────────────────────     ─────────────────          ────
ReasoningDelta  ──►  ContentBlock::Reasoning  ──►  Thinking segment
ContentDelta    ──►  ContentBlock::Text       ──►  Answer segment
ToolUse/Result  ──►  ToolUse + ToolResult     ──►  Activity segment
```

- Touches `chat_store`, stream flush buffers, `build_chat_blocks` (→ segment builder),
  tool/thinking views + collapse defaults

- Session JSON gains a new content-block variant (serde-friendly; old files keep working)

- Claude turns only get the quieter tool path; thinking UI appears when a harness emits
  `ReasoningDelta`
