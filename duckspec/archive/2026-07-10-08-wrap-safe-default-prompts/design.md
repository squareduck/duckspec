# Wrap-safe default prompt previews — Design

Two small, independent edits: soft length guidance in the shared oneshot instruction, and
a grow-with-wrap list layout in the composer defaults chrome. No parse/send path changes.

## Approach

```
oneshot result (full strings)          empty composer chrome
         │                                      │
         ▼                                      ▼
  REPLY_SUGGEST_INSTRUCTION              view_default_prompt_list
  + "each REPLY ≤ 100 chars"             drop Fixed(LINE_HEIGHT)
  (soft; no parse truncate)              text: Fill + Word wrap
         │                               row: height Shrink, align Start
         └──────── full strings ────────▶ marker ↳ on first line only
```

`chat/default-prompts` already owns parse, readiness, Tab/Enter, and the effective list.
This change only touches (1) the oneshot instruction framing and (2) how the ready list is
painted under the empty composer. Behavior of empty Enter, Tab cycle, generation
supersession, and heuristic soft-hint semantics stay as specified today.

```
crates/duckchat/src/reply_suggest.rs
  REPLY_SUGGEST_MAX_CHARS = 100
  REPLY_SUGGEST_INSTRUCTION  (+ length soft-ask)
  parse_replies              (unchanged — full strings)

crates/duckboard/src/widget/agent_chat.rs
  view_default_prompt_list   (soft-wrap + grow rows)
  view_default_prompts_loading  (unchanged)
```

## Oneshot length hint

Lives in `crates/duckchat/src/reply_suggest.rs`, shared by every harness that formats
`REPLY_SUGGEST_INSTRUCTION` before the prompt body (Claude Code worker, Grok ACP, etc.).
No harness-local copies of the instruction text.

```
// Named budget so spec, instruction, and tests share one number.
pub const REPLY_SUGGEST_MAX_CHARS: usize = 100;

// Instruction gains one soft constraint (wording approximate):
// each REPLY line's text should be at most REPLY_SUGGEST_MAX_CHARS characters.
// Prefer short slash commands and short user-voice replies.
// Do NOT hard-truncate in parse_replies when the model exceeds N.
```

Sketch:

```rust
// crates/duckchat/src/reply_suggest.rs

/// Soft per-suggestion character budget asked of the oneshot model.
/// Not enforced at parse or display — layout must still soft-wrap.
pub const REPLY_SUGGEST_MAX_CHARS: usize = 100;

pub const REPLY_SUGGEST_INSTRUCTION: &str = "… \
Prefer short user-voice replies … each REPLY text at most 100 characters. \
… Output only lines of the form REPLY: <text> …";

pub fn parse_replies(raw: &str) -> Vec<String> {
    // unchanged: REPLY: prefix, trim, drop empty, cap count at MAX_REPLIES,
    // keep full string even when longer than REPLY_SUGGEST_MAX_CHARS
    todo!()
}
```

Test surface (existing “Oneshot request framing” area):

```rust
// @spec chat/default-prompts Oneshot request framing: Length guidance is present
#[test]
fn length_guidance_is_present_in_the_instruction() {
    let inst = REPLY_SUGGEST_INSTRUCTION;
    assert!(
        inst.contains(&REPLY_SUGGEST_MAX_CHARS.to_string())
            || inst.contains("100"),
        "instruction must soft-ask ≤{REPLY_SUGGEST_MAX_CHARS} chars: {inst}"
    );
}
```

No change to `ReplySuggestionRequest`, `should_skip_model`, body line caps
(`ASSISTANT_PROMPT_MAX_LINES` / `USER_PROMPT_MAX_LINES`), or `MAX_REPLIES`.

## Defaults list layout

Lives in `view_default_prompt_list` in `crates/duckboard/src/widget/agent_chat.rs`. Today
each row is `height(Fixed(LINE_HEIGHT))` while prompt `text` has default word wrap, so
wrapped ink paints through the next row.

Target per row:

```
row  width Fill, height Shrink, align_y Start
├─ marker  Fixed(MARKER_W)   ↳ only on active row; empty otherwise
│          height Shrink / first-line only (no Fixed LINE_HEIGHT)
└─ prompt  width Fill
           wrapping Word
           size/font/color as today (active muted vs fainter inactive)
```

Column of rows: small vertical spacing (e.g. `theme::SPACING_XS`) so multi-line blocks do
not kiss. Outer padding (`CONTENT_PAD`, `CONTENT_PAD_Y`) and `width(Fill)` on the
container stay as today.

Sketch:

```rust
// crates/duckboard/src/widget/agent_chat.rs
use iced::widget::text::Wrapping;

fn view_default_prompt_list<'a>(
    prompts: &[String],
    active_idx: usize,
) -> Element<'a, Msg> {
    // MARKER_W, CONTENT_PAD unchanged
    let mut col = column![].spacing(theme::SPACING_XS);
    for (i, prompt) in prompts.iter().enumerate() {
        let line = row![
            container(text(marker) /* size, color, font */)
                .width(Length::Fixed(MARKER_W)),
            text(prompt.clone())
                .size(theme::content_size())
                .color(color)
                .font(theme::content_font())
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
        ]
        .spacing(0.0)
        .align_y(iced::Alignment::Start)
        .width(Length::Fill);
        // no .height(Length::Fixed(text_edit::LINE_HEIGHT))
        col = col.push(line);
    }
    container(col)
        .padding(/* same CONTENT_PAD_Y / CONTENT_PAD */)
        .width(Length::Fill)
        .into()
}
```

Unchanged:

- Active index, colors, `↳` only on active
- `defaults_chrome` gating (empty input, pending/loading, streaming hide)
- `view_default_prompts_loading`
- Enter / Tab handlers in `default_prompts.rs` and `interaction.rs`

Layout non-overlap is a visual property of iced’s column+wrap; no pure unit test is
required for row height. Spec can state the presentation contract; code linkage for layout
may be `test: none` or manual, while instruction length stays `test: code`.

## Spec / doc delta surface

Modify capability `chat/default-prompts` only (delta or direct edit in the change folder
per later stages):

```
| Area | Delta |
|------|--------|
| Oneshot request framing | Soft ≤100-char guidance in the shared instruction; scenario that the instruction text includes that budget |
| Presentation (new or under readiness) | Ready list: each suggestion soft-wraps within the composer width; row height follows wrapped content so consecutive suggestions do not overlap; full suggestion text remains visible; no hard truncate of display or send value |
| Parse / effective list / Enter / Tab | No semantic change — explicitly keep full strings |
```

## Decisions

- **Soft length only (N = 100)** — ask the model via instruction; do not truncate in
  `parse_replies` or the view. Alternatives: hard truncate at parse (rejected: proposal
  requires full readable previews and full send text); ellipsis single-line chrome
  (rejected: not full previews).

- **Named constant `REPLY_SUGGEST_MAX_CHARS`** — single source for the budget in code and
  tests. Alternatives: magic `"100"` only in the string (weaker coupling to tests).

- **Word wrap + grow rows** — drop fixed line height; `Wrapping::Word` + `Length::Fill` on
  text; row `align_y(Start)` so `↳` sits on the first visual line. Alternatives:
  `Wrapping::None` + clip (rejected: not full readable); Glyph wrap (possible later if
  unbreakable tokens misbehave; Word is the default for English replies).

- **Presentation requirement without layout unit tests** — document the non-overlap
  contract; rely on code review / manual check for iced layout. Alternatives: custom
  measure harness (rejected as outsized for this change).

## Risks

- **Unbreakable tokens longer than the pane** (URLs, long slash-free blobs) → Word wrap
  may not break mid-token; horizontal overflow is rare for intended reply text.
  Mitigation: still better than vertical overlap; switch to Glyph wrap only if it shows up
  in practice.

- **Three long replies grow the composer** → acceptable under full-preview intent; soft
  N=100 keeps typical height to ~1–2 lines per row on normal pane widths.

## Open questions

- none
