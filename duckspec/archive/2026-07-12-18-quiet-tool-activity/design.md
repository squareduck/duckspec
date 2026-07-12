# Quiet tool activity - Design

Align Activity chrome with Thinking’s flat muted collapsible pattern and drop tool-card
surfaces so secondary segments stay scannable without competing with User/Answer.

## Approach

No segment model, pairing, or collapse-policy changes. Presentation only in duckboard chat
view + theme, with a small `chat/transcript` doc (and light spec) delta so the calm
hierarchy stays documented.

```
BEFORE (Activity)                         AFTER (Activity = Thinking chrome)
┌──────────────────────────┐              › 2 tools · Edit, Shell
│ ▓ header bg_surface ▓    │                 ✓ Edit · …
│  body bg_base + border   │                 ✓ Shell · …
└──────────────────────────┘              (no frame, no fills)
```

```
shared secondary header
───────────────────────
chevron  ·  muted label
   │              │
collapsible   text_muted + content_font
```

Hierarchy after change:

```
| Segment  | Chrome                         | Ink              |
| -------- | ------------------------------ | ---------------- |
| User     | paper card (unchanged)         | primary          |
| Answer   | plain / last-answer band       | primary          |
| Thinking | flat header + body             | muted / secondary|
| Activity | same flat header + quiet rows  | muted / secondary|
```

## Secondary header chrome

Extract a small shared builder in `crates/duckboard/src/widget/agent_chat.rs` so Thinking
and Activity cannot drift again:

```rust
/// Flat collapsible header: chevron + muted label.
fn secondary_segment_header<'a>(
    expanded: bool,
    label: impl Into<String>,
    on_toggle: Msg,
) -> Element<'a, Msg>;
```

Row geometry matches today’s Thinking header (`SPACING_XS` vertical, `SPACING_MD`
horizontal, transparent container). Labels alone distinguish Thinking vs Activity
(`Thinking · N lines` vs `N tools · …`).

`view_thinking_block` and `view_activity_block` both call this for the header; bodies stay
kind-specific.

## Activity view

`view_activity_block` stops using the tool-card stack:

- Remove `chat_tool_card_header_open` / `_alone` / `_body` / `_frame` and the outer
  `stack![…, border_overlay]`

- Header: `secondary_segment_header` with existing `block.label`
  (`activity_collapsed_label`)

- Expanded body: same transparent `TextEdit` path as Thinking (no body style fill); apply
  `base_color(theme::text_secondary())` so expanded tool rows stay supporting detail

- Padding: match Thinking (tighter header, body without paper inset)

- `block_header_color(Activity | ToolUse)` → `text_muted` (same as Reasoning), for any
  remaining header-color callers

Legacy `BlockKind::ToolUse` / `ToolResult` still route through `view_activity_block` —
they inherit quiet chrome automatically.

## Theme cleanup

`chat_tool_card_frame`, `chat_tool_card_header_open`, `chat_tool_card_header_alone`, and
`chat_tool_card_body` in `crates/duckboard/src/theme.rs` are Activity-only today — delete
after the view no longer calls them. User card and last-answer band styles stay.

## Capability delta (`chat/transcript`)

Document the presentation hierarchy update (Activity is flat secondary chrome like
Thinking, not a framed card). Segment construction, pairing, collapse defaults, and labels
stay as-is.

```
duckspec/changes/quiet-tool-activity/caps/chat/transcript/
  doc.delta.md   — chrome hierarchy (flat secondary headers)
  spec.delta.md  — only if a testable presentation requirement is worth locking
                   (otherwise doc + view code; no new segment scenarios)
```

Automated coverage stays where it already lives (segment labels, pairing). Chrome is
visual; manual check of light/dark settled + live Activity is part of steps later.

## Impact

- duckboard-only UI/theme; no duckpond / CLI / harness changes
- No session storage or block-kind model changes
- Dead theme helpers removed
- `chat/transcript` doc (and possibly thin spec) delta on archive

## Decisions

- **No kind icons** — labels alone distinguish Thinking vs Activity. Alternative: small
  SVG icons next to labels (tried; rejected as distracting after visual check).

- **Shared header helper** — one row builder for Thinking + Activity. Alternative:
  copy-paste Thinking chrome into Activity (rejected: re-divergence risk).

- **Secondary body ink on expanded Activity** — same supporting role as Thinking body.
  Alternative: keep primary ink on tool rows (rejected for hierarchy; expanded detail
  should not re-assert primary weight).

- **Delete tool-card theme styles** — no remaining callers. Alternative: leave unused
  (rejected: dead API surface).

## Risks

- **Expanded tool output harder to scan without card contrast** → secondary ink still
  legible; status glyphs (`●` / `✓` / `✗`) remain; card was never the readability win for
  monospace rows.

- **Doc-only vs testable spec** → prefer doc for pure paint; add a spec scenario only if a
  pure function is worth unit-testing.
