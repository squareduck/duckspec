# Grok session attachments

Teach the grok harness to resolve image (and non-image) `attach:` markers on a turn into
ACP multi-block prompts so the model actually receives attachment payloads, matching what
Claude already does.

## Motivation

Duckboard already puts bytes in `TurnRequest.attachments` and embeds
`[label](attach:<id>)` in the prompt. Claude walks those links and interleaves image
content blocks; Grok only sends a single text block, so the agent sees dead markdown and
never the image. Grok is the default harness — this is a basic parity gap users hit
whenever they paste a screenshot.

## Scope

```
caps/
└── harness/
    └── grok/          (modified — prompt assembly + wire format)
        ├── spec.md
        └── doc.md
```

### New capabilities

- none

### Modified capabilities

- `harness/grok` — when running a turn, walk the prompt for `attach:` links, interleave
  resolved image bytes as ACP image content blocks (and non-image fallbacks as text), and
  send a multi-block `session/prompt` instead of a single text block

### Out of scope

- Selection-context chips (already prepended as text; already work on Grok)

- Duckboard paste/UI / attachment storage (already correct)

- Claude harness behavior (already works)

- Extracting a shared public capability for attachment assembly (internal refactor only if
  convenient)

- Audio / embedded-resource ACP content types

- Advertising or gating on agent `image` prompt capability in the UI

## Impact

```
TurnRequest.prompt + attachments
        │
        ▼
   grok harness (changed)
        │  assemble multi-block ACP prompt
        ▼
   session/prompt  [ text | image | text … ]
        │
        ▼
   grok agent (sees real images)
```

- Touches only **duckchat** grok path (`assemble_prompt` → content-block assembly;
  `AcpTurn::prompt` multi-block wire shape)

- No duckboard API changes; no breaking provider surface beyond using fields already on
  `TurnRequest`

- ACP image shape differs from Claude’s (`mimeType`/`data` vs Anthropic `source`) —
  Grok-side encoding only
