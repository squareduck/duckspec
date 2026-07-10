# Obvious chip tones and bottom pin — Design

Scannable auto-message chrome by role (blue numbered options, green/red enter and reject)
with a dual green ⌘↩ chip when multi-option lifecycle owns enter, and a viewport-driven
spacer that pins chrome above the composer when history is short.

## Approach

```
ObviousChrome                    // composition + keys UNCHANGED
        │
        ▼
pure display helpers             // dual? friendly enter label? pad height?
        │
        ├── dual_enter_lifecycle(chrome) → bool
        ├── lifecycle_enter_chip_label("/ds-apply") → "⌘↩  Apply"
        └── chrome_bottom_pad(viewport_h, content_h, prev_pad) → f32
        │
        ▼
view_obvious_chrome              // tone matrix + optional dual row
        │
        ▼
chat_col
  [transcript blocks…]
  [streaming dots?]
  [Space height = chrome_top_pad]   // 0 when content already fills viewport
  [obvious chrome chips]
        ▲
        │
ChatScrolled(viewport) ──▶ store bounds.height, content_bounds.height
                           recompute chrome_top_pad (pure)
```

Boundaries:

- **In:** `obvious_bubble` display helpers, `theme` chip styles, `agent_chat` chrome view,
  `AgentSession` pad/viewport fields + `ChatScrolled` update.

- **Out:** `build_obvious_chrome` composition table; key resolution (`resolve_cmd_*`);
  oneshot / default-prompts; user-message card style; overlay/fixed chrome; auto-messages
  setting.

User messages stay white/paper (`chat_user_card`). Numbered multi-option chips get quiet
light-blue (~8%). Enter path stays green (~8%); reject stays red (~8%).

## Display pure helpers

Module: `crates/duckboard/src/obvious_bubble.rs`.

Composition, visibility, and key resolution stay as today. Add display-only helpers so
dual-enter rules and bottom-pad math are unit-testable without iced.

```rust
// crates/duckboard/src/obvious_bubble.rs

/// True when multi-option lifecycle owns ⌘↩ and should render twice:
/// blue numbered row for lifecycle[0], plus green enter chip at the bottom.
///
/// Multi = more than one lifecycle option and no affirm.
/// Single lifecycle (e.g. /ds-explore) and affirm-only (Commit / Create change)
/// stay a single green chip.
pub fn dual_enter_lifecycle(chrome: &ObviousChrome) -> bool {
    todo!()
}

/// Strip leading `/ds-` (or `ds-`) and title-case the remainder for the enter
/// dual chip's action text. Examples: `/ds-apply` → `Apply`, `/ds-followup` →
/// `Followup`. Unknown shapes: best-effort trim of a leading `/` then title-case.
pub fn lifecycle_friendly_name(action: &str) -> String {
    todo!()
}

/// Key-first enter dual label, e.g. `⌘↩  Apply`.
pub fn lifecycle_enter_chip_label(action: &str) -> String {
    todo!()
}

/// Spacer height above chrome so chips sit at the bottom of the chat viewport
/// when natural content is shorter than the viewport.
///
/// `content_h` is the laid-out scroll content height *including* the previous
/// spacer. Subtract `prev_pad` to recover natural height and avoid feedback
/// oscillation.
///
/// `max(0, viewport_h - (content_h - prev_pad))`
pub fn chrome_bottom_pad(viewport_h: f32, content_h: f32, prev_pad: f32) -> f32 {
    todo!()
}
```

Send text on dual-chip click remains the original lifecycle string (`/ds-apply`), not the
friendly name. ⌘↩ resolution is unchanged (`resolve_cmd_enter`).

## Chip tones

Module: `crates/duckboard/src/theme.rs` + view-local `ObviousChipTone`.

Rename the multi-option non-enter role from neutral grey to numbered blue. Keep the same
base (`chat_obvious_chip_neutral` muted paper) and the same ~8% mix recipe used by
enter/reject, mixing `accent()` (Catppuccin blue) instead of green/red.

```rust
// crates/duckboard/src/theme.rs

/// Quiet light-blue chip — multi-option numbered lifecycle (⌘1…⌘n).
/// Same ~8% tint strength as enter/reject.
pub fn chat_obvious_chip_numbered(_theme: &Theme) -> container::Style {
    todo!() // start from chat_obvious_chip_neutral; mix accent() at 0.08
}

// chat_obvious_chip_enter / chat_obvious_chip_reject — unchanged recipe
// chat_obvious_chip_neutral — remains the untinted base for the mix helpers
```

View enum:

```rust
// crates/duckboard/src/widget/agent_chat.rs

enum ObviousChipTone {
    /// Multi-option numbered lifecycle chips.
    Numbered,
    /// Affirm, dual enter lifecycle, or single-option lifecycle/affirm.
    Enter,
    /// Reject.
    Reject,
}
```

Tone matrix:

```
| Chrome shape                         | Lifecycle chips              | Bottom / gate              |
|--------------------------------------|------------------------------|----------------------------|
| lifecycle.len() > 1, no affirm       | all Numbered (blue)          | dual Enter: ⌘↩ Friendly    |
| lifecycle + affirm (any len)         | all Numbered (blue)          | affirm Enter + decline Red |
| lifecycle.len() == 1, no affirm      | one Enter (green)            | —                          |
| affirm only (Commit / Create change) | —                            | one Enter (green)          |
```

Single-option lifecycle keeps today's label form (`⌘1  /ds-explore`), green tone — not
dual, not blue.

## Chrome view

Module: `crates/duckboard/src/widget/agent_chat.rs` — `view_obvious_chrome`.

```rust
fn view_obvious_chrome<'a>(
    chrome: &'a crate::obvious_bubble::ObviousChrome,
) -> Element<'a, Msg> {
    // let dual = dual_enter_lifecycle(chrome);
    // for each lifecycle[i]:
    //   label = lifecycle_chip_label(i+1, action)
    //   tone  = Enter if !dual && chrome.affirm.is_none() && i == 0
    //           else if dual || chrome.affirm.is_some() { Numbered for multi path }
    //           else Numbered or Enter per matrix above
    // if dual:
    //   push Enter chip:
    //     label  = lifecycle_enter_chip_label(lifecycle[0])
    //     action = lifecycle[0].clone()   // send /ds-apply, not "Apply"
    // then existing affirm/decline gate row (unchanged structure)
    todo!()
}
```

Dual row order (multi, no affirm):

```
┌─────────────────────────────┐  blue
│ ⌘1  /ds-apply               │
│ ⌘2  /ds-review              │  blue
│ ⌘3  /ds-followup            │  blue
│ ⌘↩  Apply                   │  green — bottom; click/⌘↩ send /ds-apply
└─────────────────────────────┘
```

With affirm present, no dual lifecycle row; green is Confirm/Commit/Create change as
today.

## Bottom pin spacer

Problem: `Length::Fill` inside scrollable content does not expand to the viewport — the
content column sizes to its children. Pinning needs an **explicit pixel** spacer derived
from the last known scroll viewport.

### Session fields

```rust
// crates/duckboard/src/area/interaction.rs — AgentSession

/// Last chat scrollable viewport height (logical px) from `ChatScrolled`.
pub chat_viewport_height: Option<f32>,
/// Last scroll content height including chrome pad (logical px).
pub chat_content_height: Option<f32>,
/// Spacer above obvious chrome so short history pins chips above the composer.
/// Ephemeral — recomputed on scroll notifications; not persisted.
pub chrome_top_pad: f32,
```

Initialize `chrome_top_pad` to `0.0`; heights to `None`.

### Update path

On `agent_chat::Msg::ChatScrolled(viewport)` (existing handler in `area/interaction.rs`),
after stick-to-bottom bookkeeping:

```rust
// bounds / content_bounds already read for stick-to-bottom
ax.chat_viewport_height = Some(bounds.height);
ax.chat_content_height = Some(content.height);
ax.chrome_top_pad = crate::obvious_bubble::chrome_bottom_pad(
    bounds.height,
    content.height,
    ax.chrome_top_pad,
);
```

When chrome is not visible, treat pad as `0.0` (no spacer in the column) so heights stay
meaningful for the next show.

### View path

```rust
// crates/duckboard/src/widget/agent_chat.rs — view()

// after transcript blocks (+ streaming indicator)
// if chrome_visible:
//   if chrome_top_pad > 0.0:
//     chat_col.push(Space::new().height(chrome_top_pad).width(Length::Fill))
//   chat_col.push(view_obvious_chrome(obvious_chrome))
```

`view` needs the pad value threaded in (new argument on `agent_chat::view`, or read from a
small chrome layout context passed by callers). Prefer an extra `chrome_top_pad: f32`
argument next to `obvious_chrome` / `auto_messages` so the widget stays free of
`AgentSession`.

First frame before any `ChatScrolled`: pad is `0` (chips may flash at the top once).
Acceptable; the next scroll/layout notification pins them. No overlay.

When natural content (messages + chrome without pad) ≥ viewport, pad is `0` and chips
follow the last message in document order — same as today for long chats.

## Decisions

- **Dual only when `lifecycle.len() > 1` and affirm is absent** — matches “single option
  stays one green chip” (explore, sole propose, Commit, Create change). Alternatives: dual
  whenever lifecycle[0] owns ⌘↩ including single (rejected: clutters explore/propose);
  dual whenever multi *options including gate* (rejected: affirm already is the green
  enter row).

- **Dual label is key-first friendly name** (`⌘↩  Apply`) — same binding pattern as
  Confirm; send text remains `/ds-…`. Alternatives: plain `Apply` without hotkey
  (rejected: inconsistent with other chips); `⌘↩  /ds-apply` (rejected: loses the
  scannable plain verb).

- **Bottom pin via measured pad, not overlay** — chips stay in the scroll column so long
  transcripts scroll them with history. Alternatives: fixed chrome above composer outside
  scroll (rejected: overlays / leaves history); `Length::Fill` spacer only (rejected:
  ineffective inside scrollable content).

- **Blue tint reuses enter/reject ~8% recipe with `accent()`** — quiet third category.
  Alternatives: stronger blue fill (rejected: too button-like); leave multi non-enter as
  neutral grey (rejected: proposal goal).

## Risks

- **Pad feedback loop / jitter** → pure `chrome_bottom_pad` subtracts `prev_pad` from
  content height; unit-test stable fixed points (short content → positive pad; tall
  content → 0).

- **First paint before viewport known** → pad starts at 0; one-frame top placement
  possible. Mitigation: accept; optional follow-up could seed height from a layout
  operation if it proves noisy.

- **Friendly-name edge cases** (`/ds-followup` → `Followup`) → simple strip + title-case;
  no special-case dictionary unless UX complains later.

## Open questions

None.
