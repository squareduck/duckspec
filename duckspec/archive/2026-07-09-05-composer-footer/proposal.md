# Composer footer

Redesign the chat input meta strip into a light, paper-blended toolbar (no border), and
pin the few honest, testable rules behind a minimal capability.

## Motivation

At small chat widths the footer feels heavy and sometimes wrong: a chrome-heavy model
control, a long `used / max (%)` string, and a “resends full history” label even when
there is nothing to resend. Those problems share one surface — the meta strip under the
prompt — so density, honesty, and quiet chrome should be fixed together without losing the
paper blend with the input.

## Scope

```
caps/
└── chat/
    ├── persistence/
    ├── transcript/
    └── composer-footer/   ← NEW (behavior only)
```

### New capabilities

- `chat/composer-footer` — Meta strip under the chat prompt with three testable rules:

  1. **Resend hint** only when the next send would actually resend history (no resumable
     session **and** a non-empty transcript)

  2. **Usage readout** progressive: **% only** when cool; full `used / max` (with %) when
     fill is hot (≥75%, same band as the existing warning color)

  3. **Closed model label** short display name; the open menu may still show harness
     grouping elsewhere

Visual chrome (paper blend, no border, lightweight model control) is implementation detail
— not capability requirements.

### Modified capabilities

- None. Context-fill math stays under `harness/model-picker`; this change only owns how
  the composer presents usage and related hints.

### Out of scope

- Transcript rendering, send/queue/priming, and session resume mechanics
- New models or harnesses
- Deltas to `harness/model-picker` fill-computation requirements
- Layout or visual snapshot tests for spacing and chrome

## Impact

Duckboard-only: composer meta strip in the chat input area, plus small pure helpers for
resend visibility, usage formatting, and closed model labels. No duckpond, CLI, or API
surface changes. Existing model-picker fill scenarios remain valid.
