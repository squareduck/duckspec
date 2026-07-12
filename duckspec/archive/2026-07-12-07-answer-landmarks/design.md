# Answer landmarks - Design

Full-width band on the latest Answer block plus ⌘-arrow transcript navigation, built on
existing chat block widget ids and scroll operations — duckboard-only, no harness or
library changes.

## Approach

Two thin layers over the calm transcript: **paint** the last `BlockKind::Assistant` with a
full-width surface, and **jump** the chat scrollable using the same layout-id
infrastructure find already uses.

```
blocks[] ──► last_assistant_idx ──► view_prose_block (band style)
    │
    └──► answer_block_indices[] ──► ⌘←/→ resolve ──► scroll_block_to_top(chat-block-i)
                                        │
⌘↑ ──► AbsoluteOffset { y: 0 }          │
⌘↓ ──► snap_to_end + stick_to_bottom    │
                                        ▼
                         clear stick on leave-bottom jumps
                         (same as find's jump_to_current)
```

```
| Concern | Where it lives |
| --- | --- |
| Tint surface | `theme` + `agent_chat::view_prose_block` |
| “Which block is last Answer?” | pure helper over `&[Block]` |
| Answer anchor list | pure helper over `&[Block]` |
| ⌘-arrow eligibility | `keybinds` (+ existing modal early-outs in `main`) |
| Scroll tasks | reuse `find::scroll_block_to_top` / `snap_to_end` / `scroll_to` |
```

No new persistence. No change to segment construction (`build_transcript_segments` stays
as-is).

## Last-Answer band

In `agent_chat::view`, when rendering blocks, compute:

```rust
fn last_assistant_block_idx(blocks: &[Block]) -> Option<usize> {
    blocks.iter().rposition(|b| b.kind == BlockKind::Assistant)
}
```

Pass `is_last_answer` into `view_prose_block`. For `BlockKind::Assistant` when true, wrap
the existing padded content in a **full-width** container with a new theme style — **no
border, no radius, no horizontal inset** (unlike `chat_user_card`, which is intentionally
card-shaped).

```rust
// theme.rs sketch
pub fn bg_chat_last_answer() -> Color { /* one step from bg_chat_area toward bg_surface */ }

pub fn chat_last_answer_band(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(bg_chat_last_answer().into()),
        ..Default::default() // no Border
    }
}
```

Empty Answer blocks already render as `Space` (`view_prose_block` early-return when
`lines` empty) — no band until Answer text exists, including mid-stream once the live
Answer has content.

Older Answers keep today’s plain padding on `bg_chat_area`. Thinking / Activity / User
unchanged.

## Answer navigation

### Pure index helpers (`agent_chat` or small unit next to it)

```rust
fn answer_block_indices(blocks: &[Block]) -> Vec<usize> {
    blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| b.kind == BlockKind::Assistant)
        .map(|(i, _)| i)
        .collect()
}

/// `from` = block index of the Answer treated as current; None if unknown.
fn prev_answer_idx(anchors: &[usize], from: Option<usize>) -> Option<usize> { /* no wrap */ }
fn next_answer_idx(anchors: &[usize], from: Option<usize>) -> Option<usize> { /* no wrap */ }
```

### Current Answer for ⌘←/→

Prefer **viewport-relative current**, not a sticky keyboard cursor alone:

- Collect content-y of each Answer block via a small Operation (same untranslated-layout
  approach as `ScrollBlockToTop` in `widget/find.rs`).

- With `last_chat_offset_y` as viewport top: **current** = last Answer whose content-y ≤
  offset (+ small epsilon); if none, first Answer.

- If `stick_to_bottom` (user is glued to the end): treat **current** as the last Answer so
  ⌘← steps to the previous reply.

Then scroll with existing:

```rust
widget::find::scroll_block_to_top(
    agent_chat::CHAT_SCROLLABLE_ID,
    widget::find::chat_block_widget_id(block_idx),
)
```

Set `chat_scroll_overridden = true`, clear `stick_to_bottom` / `pending_snap_to_bottom` on
top and prev/next (mirror `jump_to_current`). On **⌘↓**, `snap_to_end` and set
`stick_to_bottom = true` so streaming continues to follow.

⌘↑: `scroll_to(..., AbsoluteOffset { x: 0.0, y: 0.0 })`.

If prev/next has no target → no-op (no wrap). Zero Answers → all four shortcuts still run
top/bottom; left/right no-op.

### Keybind wiring

New resolver in `keybinds.rs` (focus-aware, like existing actions):

```rust
pub enum ChatLandmarkAction {
    HistoryTop,
    HistoryBottom,
    PrevAnswer,
    NextAnswer,
}

/// ⌘↑/↓/←/→ when the chat transcript is the active interaction.
pub fn keybind_chat_landmarks(state: &State) -> Option<ChatLandmarkAction>
```

Gate: chat tab visible + active session; **not** terminal-focused. Modal ownership stays
in `main`’s existing early returns (find, file finder, quick idea, project picker, …) so
those keep bare and modified arrows. Do **not** put these in `handle_agent_chat_key`
(completion / esc / fast-response only).

Dispatch in `main`’s `KeyPress` arm after modal handlers, when chat is active — works with
composer focused (⌘-arrows only; bare arrows stay with TextEdit).

## Impact

- **duckboard only** (`theme`, `agent_chat`, `keybinds`, `main` key cascade; optional
  small Operation helper next to `find::scroll_block_to_top`)

- No duckpond / ds / harness / persistence changes

- Spec surface likely extends **chat/transcript** presentation and adds chat landmark
  keyboard behavior (exact cap split left to `/ds-spec`)

- No migrations or public API

## Decisions

- **Full-width band, not card** - container background edge-to-edge; reject
  border/radius/user-card reuse so Answers do not look like second-class user bubbles.

- **Last Answer only** - tint tracks `rposition` of `BlockKind::Assistant`; not whole
  agent turn or exchange.

- **Answer tops only for ←/→** - anchors are Assistant blocks; Thinking / Activity never
  receive jumps.

- **Viewport-derived current** - prev/next relative to scroll position (+ stick-to-bottom
  ⇒ last Answer). Alternative: only a keyboard cursor (rejected: desyncs after manual
  scroll).

- **Reuse find scroll Operation** - no parallel pixel math for block tops.

- **Keybinds module for eligibility** - keeps `main` thin and matches other cmd shortcuts.

## Risks

- **Tint too strong or invisible** → one-step palette step; tune against light/dark in
  theme only.

- **Layout Operation misses ids** (collapsed/empty) → empty Answers are not anchors;
  collapsed Thinking above does not change Answer block ids.

- **Streaming layout churn** while jumping → same stick/override path as find; jumps
  deliberately unstick from bottom.

## Open questions

- Exact mix for `bg_chat_last_answer` (e.g. 50% toward `bg_surface` vs full `bg_surface`)
  — pick during implement by eye; no product fork.
