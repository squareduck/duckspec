# Post-implementation review: Markdown table render

Reviewed `md-table-render` end-to-end against proposal, design, caps, steps, and code. The
pure kernel and hybrid wiring are sound and largely faithful; freeze is blocked by a real
integration hole (find highlights on table bands) and avoidable craft debt in wrap +
layout thrash.

## Scope

Post-implementation, full chain:

- `proposal.md`, `design.md`
- new cap `caps/editor/md-table` (spec + doc)
- steps 01–04 (all complete)
- `crates/duckboard/src/widget/md_table.rs`
- hybrid path in `crates/duckboard/src/widget/text_edit/render.rs`
- chat enablement in `crates/duckboard/src/widget/agent_chat.rs`

Deepest layer: code. Unit tests for the kernel and hybrid layout pass.

## Findings

### Find/search highlights never paint on table bands — soundness/major

`render.rs` takes an early `continue` after painting a table visual row (`~1470–1571`), so
the later match-range pass (`highlight_ranges` / `current_highlight` at `~1615–1684`)
never runs for those lines.

Chat enables both `.md_tables(true)` and `.highlights(...)` on the same `TextEdit`
(`agent_chat.rs:777–781`, `:854–858`). For read-only chat blocks the caret is suppressed
and **highlights are the only find affordance** (see the cursor comment at
`render.rs:1915–1919`). A match that lands inside a table cell is therefore invisible
while still “found” by the finder.

Recommended action: paint fragment-clipped match quads on the table band path (same
byte→fragment mapping already used by `paint_table_selection`), then continue. Mirror the
selection paint style so find and selection stay aligned.

### Cell soft-wrap reimplements prose wrap — quality/major

`md_table::soft_wrap_cell` (`md_table.rs:424–467`) is the same space-preferring char-grid
wrap as `wrap_line_starts` (`render.rs:322–349`), differing only in output shape (fragment
ranges vs start offsets). Design already called for the same spirit as `wrap_line_starts`;
shipping a second copy means break-at-space bugs get fixed twice or drift.

Recommended action: extract one shared soft-wrap helper (e.g. char starts or ranges) and
implement both `wrap_line_starts` and cell fragments on top of it.

### EditorLayout recomputed on every layout / update / draw — quality/major

Design risk mitigation: *“single `EditorLayout` computed once per frame and shared by
draw, `pixel_to_pos`, and visual caret motion.”* Implementation calls
`EditorLayout::compute` independently in `layout` (`~823–824`), `update` (`~899–905`), and
`draw` (`~1303–1309`). Each pass re-scans all lines through `layout_tables`.

Under chat (`fit_content` + `md_tables`), that is full-table recognition and fit on every
mouse move and paint for every message body. Cost compounds with message length and will
show up as input latency before correctness fails.

Recommended action: cache the hybrid layout on `InternalState` keyed by
`(pane_chars, word_wrap, lines identity/version)` and reuse across layout, update, and
draw until inputs change.

### Link hover underline skipped on table bands — quality/minor

Cmd-hover still *detects* links in cells via `pixel_to_pos` → `detect_link_at`, and
cmd-click still opens them, but the underline overlay lives only on the prose path
(`render.rs:1881–1912`). Table cells get click-without-underline UX.

Recommended action: when painting a table fragment that overlaps a hovered link’s char
range, draw the same accent underline under that fragment slice.

## What went well

- **Architecture holds.** Pure `widget/md_table` + opt-in `md_tables` + hybrid
  `EditorLayout` matches the design decisions; file tabs stay line-faithful.

- **Cap ↔ kernel fidelity is tight.** Recognition, fit/overflow, separator-as- metadata,
  pipe-free fragments, and bidirectional source maps are implemented and `@spec`-linked in
  unit tests — all green.

- **Integration essentials land.** Separator contributes zero visual rows; overflow widens
  `content_width_chars` / `scroll_x`; visual up/down skip the separator; selection is
  fragment-clipped; chrome is role-driven paint only; chat/tool bodies enable the flag
  without touching duckpond or persistence.

- **Streaming choice is correct.** Incomplete tables stay prose until a full GFM block
  parses — no speculative half-grids.

## Verdict

Not ready to accept as done. The thinking (proposal → design → pure-layout cap) is sound,
and most of the code is a clean realization of that plan. What remains is not polish: find
matches disappear on the primary surface that enables tables (chat), soft-wrap is
duplicated with prose, and layout thrash contradicts the design’s single-layout-per-frame
rule. Fix the highlight gap and the wrap/layout debt before archiving; the link-underline
nit is optional.

`/ds-step` is the right next command for all of the above (no new cap behavior — editor
integration and craft inside existing design).
