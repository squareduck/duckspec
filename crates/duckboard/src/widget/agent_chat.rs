//! Agent chat widget — per-message text editors in a scrollable column.

use iced::widget::{Space, button, column, container, row, rule, scrollable, stack, text};

pub const CHAT_SCROLLABLE_ID: &str = "agent-chat-scroll";
pub const CHAT_INPUT_ID: &str = "agent-chat-input";
/// Pixels of slack at the bottom edge that still count as "stuck to bottom".
/// Small enough that one wheel notch unsticks the view, large enough to
/// absorb sub-pixel layout rounding during streaming rebuilds.
pub const STICK_TO_BOTTOM_THRESHOLD: f32 = 16.0;
use iced::{Element, Length};

use crate::agent::SlashCommand;
use crate::area::interaction::{self, SelectionContext};
use crate::chat_store::{ChatSession, ContentBlock, Role};
use crate::theme;
use crate::widget::collapsible;
use crate::widget::streaming_indicator;
use crate::widget::text_edit::{self, Block, BlockKind, EditorState};

// ── Messages ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Msg {
    /// Action from the chat input editor.
    InputAction(text_edit::EditorAction),
    SendPressed,
    CancelPressed,
    CompletionAccept,
    CompletionNext,
    CompletionPrev,
    CompletionDismiss,
    /// Action from a per-block chat text editor (index, action).
    ChatAction(usize, text_edit::EditorAction),
    /// Toggle collapse state of a block.
    ToggleCollapse(usize),
    /// Action from the queued-message read-only editor.
    QueueAction(text_edit::EditorAction),
    /// Discard the queued message (from the pill's ✕ button).
    DiscardQueue,
    /// User scrolled the chat transcript. Drives the per-session
    /// `stick_to_bottom` flag — true while the viewport is within
    /// `STICK_TO_BOTTOM_THRESHOLD` pixels of the bottom.
    ChatScrolled(scrollable::Viewport),
}

// ── Status bar info ────────────────────────────────────────────────────────

/// Data for the status bar below the chat input.
pub struct StatusInfo {
    pub is_streaming: bool,
    /// 0 = no esc pressed, 1 = one esc pressed (waiting for second).
    pub esc_count: u8,
    pub model: String,
    pub context_tokens: usize,
    pub context_max: usize,
}

// ── Completion state ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct CompletionState {
    pub visible: bool,
    pub selected: usize,
}

// ── Build blocks from session ──────────────────────────────────────────────

/// Build blocks from a chat session for the block-aware editor.
///
/// ToolUse and ToolResult pairs are merged into a single ToolUse block
/// whose label is the tool summary and whose lines are the (truncated)
/// result output.
pub fn build_chat_blocks(session: &ChatSession) -> Vec<Block> {
    // Flatten all content blocks with their role, so we can look ahead.
    let mut items: Vec<(&Role, &ContentBlock)> = Vec::new();
    for msg in &session.messages {
        for cb in &msg.content {
            items.push((&msg.role, cb));
        }
    }

    let mut blocks = Vec::new();
    let mut i = 0;
    while i < items.len() {
        let (role, cb) = items[i];
        match cb {
            ContentBlock::Text(t) => {
                let kind = match role {
                    Role::User => BlockKind::User,
                    Role::Assistant => BlockKind::Assistant,
                    Role::System => BlockKind::System,
                };
                let role_label = match role {
                    Role::User => "User",
                    Role::Assistant => "Assistant",
                    Role::System => "System",
                };
                let lines: Vec<String> = t.lines().map(String::from).collect();
                blocks.push(Block {
                    kind,
                    label: role_label.to_string(),
                    lines,
                });
            }
            ContentBlock::ToolUse { id, name, input } => {
                let summary = format_tool_summary(name, input);

                // Look ahead for a matching ToolResult and merge.
                let result_lines = if let Some((
                    _,
                    ContentBlock::ToolResult {
                        id: rid, output, ..
                    },
                )) = items.get(i + 1)
                {
                    if rid == id {
                        i += 1; // consume the result
                        truncate_output(output)
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                blocks.push(Block {
                    kind: BlockKind::ToolUse,
                    label: summary,
                    lines: result_lines,
                });
            }
            ContentBlock::ToolResult { output, .. } => {
                // Orphan result (no preceding ToolUse) — show standalone.
                let lines = truncate_output(output);
                blocks.push(Block {
                    kind: BlockKind::ToolResult,
                    label: "✓ done".to_string(),
                    lines,
                });
            }
        }
        i += 1;
    }

    // Streaming pending text.
    if session.is_streaming && !session.pending_text.is_empty() {
        let lines: Vec<String> = session.pending_text.lines().map(String::from).collect();
        blocks.push(Block {
            kind: BlockKind::Assistant,
            label: "Assistant ···".to_string(),
            lines,
        });
    }

    blocks
}

/// Truncate tool output to a reasonable number of lines, filtering
/// non-printable characters that cause rendering artifacts.
fn truncate_output(output: &str) -> Vec<String> {
    const MAX_LINES: usize = 10;
    let cleaned = strip_ansi_escapes(output);
    let cleaned = strip_tool_wrapper_tags(&cleaned);
    let all_lines: Vec<String> = cleaned
        .lines()
        .map(sanitize_line)
        .map(|s| s.trim_end().to_string())
        .collect();
    let mut lines = if all_lines.len() > MAX_LINES {
        let mut truncated = all_lines[..MAX_LINES].to_vec();
        truncated.push(format!("… ({} more lines)", all_lines.len() - MAX_LINES));
        truncated
    } else {
        all_lines
    };
    while lines.last().is_some_and(|s| s.is_empty()) {
        lines.pop();
    }
    lines
}

/// Remove ANSI CSI escape sequences (e.g. `\x1B[32m`). Parses the sequence
/// greedily through its final byte so the parameter bytes don't leak through.
fn strip_ansi_escapes(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1B' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                for next in chars.by_ref() {
                    let n = next as u32;
                    if (0x40..=0x7E).contains(&n) {
                        break;
                    }
                }
            }
            Some(']') => {
                // OSC: terminate on BEL (0x07) or ESC \\.
                let mut prev_esc = false;
                for next in chars.by_ref() {
                    if next == '\x07' {
                        break;
                    }
                    if prev_esc && next == '\\' {
                        break;
                    }
                    prev_esc = next == '\x1B';
                }
            }
            Some(_) | None => {}
        }
    }
    out
}

/// Strip `<tool_use_error>` / `<tool_use_result>` wrapper tags that some
/// agent backends emit around tool output.
fn strip_tool_wrapper_tags(input: &str) -> String {
    input
        .replace("<tool_use_error>", "")
        .replace("</tool_use_error>", "")
        .replace("<tool_use_result>", "")
        .replace("</tool_use_result>", "")
}

/// Replace remaining non-printable / non-standard-whitespace characters with a
/// space to avoid rendering rectangles in the monospace font.
fn sanitize_line(line: &str) -> String {
    line.chars()
        .map(|c| if c == '\t' || c.is_control() { ' ' } else { c })
        .collect()
}

/// Produce a short human-readable summary of a tool call.
/// E.g. `Read /src/main.rs` or `Edit /src/lib.rs`.
fn format_tool_summary(name: &str, input: &str) -> String {
    // Try to extract key fields from JSON.
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(input) {
        // Many tools have a file_path or path field — use that as the summary.
        let path = map
            .get("file_path")
            .or_else(|| map.get("path"))
            .and_then(|v| v.as_str());
        if let Some(p) = path {
            // Shorten to last 3 path components.
            let short: String = p
                .rsplit('/')
                .take(3)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("/");
            return format!("{name} {short}");
        }

        let pattern = map.get("pattern").and_then(|v| v.as_str());
        if let Some(pat) = pattern {
            let truncated = if pat.len() > 40 { &pat[..40] } else { pat };
            return format!("{name} \"{truncated}\"");
        }

        let command = map.get("command").and_then(|v| v.as_str());
        if let Some(cmd) = command {
            let truncated = if cmd.len() > 50 { &cmd[..50] } else { cmd };
            return format!("{name} `{truncated}`");
        }
    }

    name.to_string()
}

// ── View ────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    _session: &'a ChatSession,
    blocks: &'a [Block],
    editors: &'a [EditorState],
    collapsed: &'a [bool],
    input_value: &'a EditorState,
    queue_editor: Option<&'a EditorState>,
    commands: &'a [SlashCommand],
    completion: &CompletionState,
    status: StatusInfo,
    obvious_command: Option<&str>,
    pinned_selections: &'a [SelectionContext],
    tentative_selection: Option<&'a SelectionContext>,
    block_highlights: Vec<(Vec<text_edit::HighlightRange>, Option<text_edit::HighlightRange>)>,
) -> Element<'a, Msg> {
    // Chat content — scrollable column of full-width sections.
    let mut chat_col = column![]
        .spacing(theme::SPACING_XS)
        .padding([theme::SPACING_SM, 0.0]);

    let mut block_highlights = block_highlights;
    for (i, block) in blocks.iter().enumerate() {
        let is_collapsed = collapsed.get(i).copied().unwrap_or(false);
        let (ranges, current) = if i < block_highlights.len() {
            std::mem::take(&mut block_highlights[i])
        } else {
            (Vec::new(), None)
        };
        let block_el = view_block(i, block, editors.get(i), is_collapsed, ranges, current);
        // Tag each block with a stable widget id so `widget::find` can read
        // the laid-out bounds during an Operation pass and scroll the
        // matching block to the top of the viewport — bypasses all the
        // per-kind padding / wrap / collapse pixel math.
        let tagged = container(block_el)
            .id(crate::widget::find::chat_block_widget_id(i))
            .width(Length::Fill);
        chat_col = chat_col.push(tagged);
    }

    // Streaming indicator: animated pulsing dots + inline cancel hint at
    // the bottom of the transcript, visible only while the agent is
    // producing a response. The left padding (`SPACING_MD + SPACING_SM`)
    // mirrors the block-container padding + `TextEdit`'s internal
    // `CONTENT_PAD`, so the dots land at the same x as message body text.
    if status.is_streaming {
        chat_col = chat_col.push(
            container(streaming_indicator::view(status.esc_count))
                .padding([theme::SPACING_SM, theme::SPACING_MD + theme::SPACING_SM])
                .width(Length::Fill),
        );
    }

    let chat_scroll = scrollable(chat_col)
        .direction(theme::thin_scrollbar_direction())
        .style(theme::thin_scrollbar)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_scroll(Msg::ChatScrolled)
        .id(CHAT_SCROLLABLE_ID);
    let chat_area = container(chat_scroll)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::chat_area);

    // Completion popup — always rendered with the same widget type so iced's
    // tree diff preserves input focus. When hidden, the inner column is
    // empty and the background is suppressed so the popup collapses cleanly.
    // When shown it shares the chat input's "paper" bg so the popup reads as
    // a continuation of the input field with a top hairline separating it
    // from the chat transcript.
    let has_completion = completion.visible && {
        let input_text = input_value.text();
        let query = input_text.trim_start_matches('/');
        !filter_commands(commands, query).is_empty()
    };
    let completion_col = if has_completion {
        let input_text = input_value.text();
        let query = input_text.trim_start_matches('/');
        let filtered = filter_commands(commands, query);
        let mut col = column![].spacing(0.0);
        col = col.push(completion_divider());
        col = col.push(view_completion_col(
            commands,
            &filtered,
            completion.selected,
        ));
        col
    } else {
        column![].spacing(0.0)
    };
    let completion_el: Element<'a, Msg> = container(completion_col)
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme| {
            if has_completion {
                container::Style {
                    background: Some(iced::Background::Color(theme::bg_base())),
                    ..Default::default()
                }
            } else {
                container::Style::default()
            }
        })
        .into();

    // Input area — promoted to the custom TextEdit widget so prompts get
    // markdown syntax highlighting and the full editor toolkit (undo,
    // word-nav, selection). Plain Enter sends via `on_submit`; Shift+Enter
    // falls through to the default newline action.
    let mut input = text_edit::TextEdit::new(input_value, Msg::InputAction)
        .id(CHAT_INPUT_ID)
        .show_gutter(false)
        .word_wrap(true)
        .fit_content(true)
        .transparent_bg(true)
        .on_submit(Msg::SendPressed);
    if let Some(cmd) = obvious_command {
        input = input.placeholder(format!("Press Enter to run /{cmd}"));
    }

    let input_divider = rule::horizontal(1).style(|_theme: &iced::Theme| rule::Style {
        color: theme::border_color(),
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    });

    // Meta row — model + context tokens — sits inside the input container
    // below the editor, blending into the "paper" surface (à la Zed). The
    // extra `SPACING_SM` horizontal padding lines the meta text up with the
    // input's own text (container XS + TextEdit CONTENT_PAD = 12px).
    let ctx_pct = if status.context_max > 0 {
        (status.context_tokens as f32 / status.context_max as f32 * 100.0) as usize
    } else {
        0
    };
    let ctx_color = if ctx_pct >= 90 {
        theme::error()
    } else if ctx_pct >= 75 {
        theme::warning()
    } else {
        theme::text_muted()
    };
    let has_attachments =
        !pinned_selections.is_empty() || tentative_selection.is_some();
    let mut meta_inner = row![].spacing(theme::SPACING_SM).align_y(iced::Alignment::Center);
    if has_attachments {
        meta_inner = meta_inner.push(
            text("⌘R reset")
                .size(theme::font_sm())
                .color(theme::text_muted()),
        );
    }
    meta_inner = meta_inner
        .push(Space::new().width(Length::Fill))
        .push(
            text(status.model)
                .size(theme::font_sm())
                .color(theme::text_muted()),
        )
        .push(
            text(format!(
                "{} / {} ({}%)",
                format_number(status.context_tokens),
                format_number(status.context_max),
                ctx_pct,
            ))
            .size(theme::font_sm())
            .color(ctx_color),
        );
    let meta_row = container(meta_inner)
        .padding([0.0, theme::SPACING_SM])
        .width(Length::Fill);

    // Queue pill — renders above the input when a message is staged while the
    // agent is still streaming. Uses a read-only TextEdit so it matches the
    // shape of regular chat messages.
    let queue_el: Element<'a, Msg> = match queue_editor {
        Some(ed) => {
            let editor = text_edit::TextEdit::new(ed, Msg::QueueAction)
                .show_gutter(false)
                .word_wrap(true)
                .read_only(true)
                .fit_content(true)
                .transparent_bg(true);
            let close_btn = button(
                text("×")
                    .size(theme::content_size())
                    .color(theme::text_muted()),
            )
            .on_press(Msg::DiscardQueue)
            .padding([0.0, theme::SPACING_XS])
            .style(|_theme, _status| iced::widget::button::Style {
                background: None,
                ..Default::default()
            });
            let label = text("Queued (enter to interrupt and send, backspace to cancel)")
                .size(theme::font_sm())
                .color(theme::text_muted());
            let header_row = row![
                container(label).width(Length::Fill),
                container(close_btn).align_y(iced::Alignment::Start),
            ]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Alignment::Center);
            let pill_col = column![header_row, container(editor).width(Length::Fill)]
                .spacing(theme::SPACING_XS);
            container(pill_col)
                .padding([theme::SPACING_SM, theme::SPACING_MD])
                .width(Length::Fill)
                .style(theme::chat_queued_card)
                .into()
        }
        None => Space::new().into(),
    };

    // Selection-context chips: pinned first, then the live tentative slot.
    // Iced 0.14's `Row::wrap()` lays children out across multiple lines
    // when they overflow horizontally, so a long chip set grows upward
    // above the input rather than forcing a horizontal scroll. Tab-source
    // labels are abbreviated to filename + minimal disambiguating
    // parents — long paths would otherwise get truncated by ellipsis.
    let attachments_el: Element<'a, Msg> = if has_attachments {
        let mut all: Vec<&SelectionContext> = pinned_selections.iter().collect();
        if let Some(t) = tentative_selection {
            all.push(t);
        }
        let labels = interaction::chip_labels_abbreviated(&all);
        let pinned_count = pinned_selections.len();
        let mut chips: Vec<Element<'a, Msg>> = Vec::with_capacity(all.len());
        for (i, label) in labels.into_iter().enumerate() {
            let tentative = i >= pinned_count;
            chips.push(view_selection_chip(label, tentative));
        }
        let wrapped = iced::widget::Row::with_children(chips)
            .spacing(theme::SPACING_XS)
            .align_y(iced::Alignment::Center)
            .wrap()
            .vertical_spacing(theme::SPACING_XS);
        container(wrapped)
            .padding([0.0, theme::SPACING_SM])
            .width(Length::Fill)
            .into()
    } else {
        Space::new().into()
    };

    // Horizontal padding here sums with TextEdit's internal CONTENT_PAD (8px)
    // to land the input's text at the same 12px the chat headers and the
    // completion rows use.
    let input_row = container(
        column![queue_el, attachments_el, input, meta_row].spacing(theme::SPACING_XS),
    )
    .padding([theme::SPACING_SM, theme::SPACING_XS])
    .width(Length::Fill)
    .style(theme::chat_input);

    column![chat_area, completion_el, input_divider, input_row]
        .height(Length::Fill)
        .into()
}

/// Render a single chat block, Zed-style:
///
/// - **User**: bordered card on the "paper" surface (no label, no chevron).
/// - **Assistant / System**: plain text flowing directly on the chat
///   background — no header, no chevron, no card.
/// - **Tool use / tool result**: bordered card with a chevron + label header
///   (collapsible), visually distinct from message text.
fn view_block<'a>(
    idx: usize,
    block: &'a Block,
    editor: Option<&'a EditorState>,
    collapsed: bool,
    hl_ranges: Vec<text_edit::HighlightRange>,
    hl_current: Option<text_edit::HighlightRange>,
) -> Element<'a, Msg> {
    let is_tool = matches!(block.kind, BlockKind::ToolUse | BlockKind::ToolResult);
    if is_tool {
        return view_tool_block(idx, block, editor, collapsed, hl_ranges, hl_current);
    }

    // User / Assistant / System: no header, no chevron.
    let has_content = !block.lines.is_empty();
    if !has_content {
        return Space::new().into();
    }
    let Some(ed) = editor else {
        return Space::new().into();
    };

    let content = text_edit::TextEdit::new(ed, move |action| Msg::ChatAction(idx, action))
        .show_gutter(false)
        .word_wrap(true)
        .read_only(true)
        .fit_content(true)
        .transparent_bg(true)
        .highlights(hl_ranges, hl_current);

    let padded = container(content)
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .width(Length::Fill);

    match block.kind {
        BlockKind::User => container(padded.style(theme::chat_user_card))
            .padding([0.0, theme::SPACING_SM])
            .width(Length::Fill)
            .into(),
        _ => padded.into(),
    }
}

/// Tool-use / tool-result rendering: framed card with a quieter header
/// surface and a `bg_base` body that matches the user bubble. Clicking the
/// header toggles `collapsed`.
fn view_tool_block<'a>(
    idx: usize,
    block: &'a Block,
    editor: Option<&'a EditorState>,
    collapsed: bool,
    hl_ranges: Vec<text_edit::HighlightRange>,
    hl_current: Option<text_edit::HighlightRange>,
) -> Element<'a, Msg> {
    let label_color = block_header_color(block.kind);
    let has_content = !block.lines.is_empty();
    let body_shown = has_content && !collapsed && editor.is_some();

    // Tool headers use the content (monospace) font so tool names and
    // paths read like code, matching the highlighted result body below.
    let header_content: Element<'a, Msg> = if has_content {
        let label = text(&block.label)
            .size(theme::content_size())
            .font(theme::content_font())
            .color(label_color);
        let header_row = row![collapsible::chevron(!collapsed), label]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Alignment::Center);
        button(header_row)
            .on_press(Msg::ToggleCollapse(idx))
            .padding(0.0)
            .style(|_theme, _status| iced::widget::button::Style {
                background: None,
                ..Default::default()
            })
            .into()
    } else {
        text(&block.label)
            .size(theme::content_size())
            .font(theme::content_font())
            .color(label_color)
            .into()
    };

    let header_style = if body_shown {
        theme::chat_tool_card_header_open
    } else {
        theme::chat_tool_card_header_alone
    };
    let header = container(header_content)
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .width(Length::Fill)
        .style(header_style);

    let mut card_col = column![header].width(Length::Fill);

    if body_shown && let Some(ed) = editor {
        let body = container(
            text_edit::TextEdit::new(ed, move |action| Msg::ChatAction(idx, action))
                .show_gutter(false)
                .word_wrap(true)
                .read_only(true)
                .fit_content(true)
                .transparent_bg(true)
                .highlights(hl_ranges, hl_current),
        )
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .width(Length::Fill)
        .style(theme::chat_tool_card_body);
        card_col = card_col.push(body);
    }

    // Outer: stack the column underneath a transparent-bg, border-only
    // overlay so the 1px frame draws on top of the header/body surfaces.
    // (A plain outer container doesn't work here: children fill the full
    // bounds and cover the parent's border stroke, so the frame needs to
    // sit *above* the children in draw order.)
    let border_overlay = container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::chat_tool_card_frame);
    let framed = stack![card_col, border_overlay];

    container(framed)
        .padding([0.0, theme::SPACING_SM])
        .width(Length::Fill)
        .into()
}

/// One selection-context chip — a small bordered label sitting above the
/// chat input. `tentative` chips use a muted border to signal "not yet
/// pinned (Cmd-K to keep)"; pinned chips use the primary border color.
fn view_selection_chip<'a>(label: String, tentative: bool) -> Element<'a, Msg> {
    let style = if tentative {
        theme::selection_chip_tentative
    } else {
        theme::selection_chip_pinned
    };
    let color = if tentative {
        theme::text_secondary()
    } else {
        theme::text_primary()
    };
    container(
        text(label)
            .size(theme::font_sm())
            .color(color)
            .wrapping(iced::widget::text::Wrapping::None),
    )
    .padding([2.0, theme::SPACING_SM])
    .style(style)
    .into()
}

/// Header label color for a block kind (re-exported from text_edit for convenience).
fn block_header_color(kind: BlockKind) -> iced::Color {
    match kind {
        BlockKind::User => theme::accent(),
        BlockKind::Assistant => theme::text_secondary(),
        // Tool blocks sit in a neutral palette (primary text) so tool names
        // stay legible without competing with the accent-colored User card.
        BlockKind::ToolUse => theme::text_primary(),
        BlockKind::ToolResult => theme::text_secondary(),
        BlockKind::System => theme::text_muted(),
    }
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

// view_status_bar removed: model + context now blend into the input area
// (see `view`), and stream state is conveyed by the streaming indicator.

// ── Completion popup ────────────────────────────────────────────────────────

fn view_completion_col<'a>(
    commands: &'a [SlashCommand],
    filtered: &[(usize, i32)],
    selected: usize,
) -> iced::widget::Column<'a, Msg> {
    let mut items = column![].spacing(0.0);
    for (i, &(cmd_idx, _score)) in filtered.iter().enumerate() {
        let cmd = &commands[cmd_idx];
        let is_selected = i == selected;
        let label = row![
            text(format!("/{}", cmd.name))
                .size(theme::font_sm())
                .color(theme::text_primary()),
            Space::new().width(theme::SPACING_SM),
            text(&cmd.description)
                .size(theme::font_sm())
                .color(theme::text_muted()),
        ]
        .align_y(iced::Alignment::Center);
        items = items.push(
            container(label)
                .width(Length::Fill)
                .padding([theme::SPACING_XS, theme::SPACING_MD])
                .style(move |_theme: &iced::Theme| {
                    if is_selected {
                        container::Style {
                            background: Some(iced::Background::Color(theme::bg_list_hover())),
                            ..Default::default()
                        }
                    } else {
                        container::Style::default()
                    }
                }),
        );
    }
    items
}

/// Hairline separator used at the top of the completion popup so it reads
/// as a distinct surface sitting above the chat transcript.
fn completion_divider<'a>() -> Element<'a, Msg> {
    rule::horizontal(1)
        .style(|_theme: &iced::Theme| rule::Style {
            color: theme::border_color(),
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}

// ── Fuzzy matching ──────────────────────────────────────────────────────────

/// Filter and score commands by fuzzy-matching `query` against command names.
/// Returns `(index_into_commands, score)` sorted by descending score.
pub fn filter_commands(commands: &[SlashCommand], query: &str) -> Vec<(usize, i32)> {
    let mut matches: Vec<(usize, i32)> = commands
        .iter()
        .enumerate()
        .filter_map(|(i, cmd)| fuzzy_score(query, &cmd.name).map(|s| (i, s)))
        .collect();
    matches.sort_by(|a, b| b.1.cmp(&a.1));
    matches
}

/// Subsequence fuzzy match. Returns `None` if `query` is not a subsequence of
/// `target`, otherwise a score (higher = better).
fn fuzzy_score(query: &str, target: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let query_lower: Vec<char> = query.to_lowercase().chars().collect();
    let target_lower: Vec<char> = target.to_lowercase().chars().collect();
    let mut qi = 0;
    let mut score = 0i32;
    let mut prev_match = false;

    for (i, &ch) in target_lower.iter().enumerate() {
        if qi < query_lower.len() && ch == query_lower[qi] {
            qi += 1;
            score += 1;
            if i == 0 {
                score += 3; // bonus for matching start
            }
            if prev_match {
                score += 2; // bonus for consecutive
            }
            prev_match = true;
        } else {
            prev_match = false;
        }
    }

    if qi == query_lower.len() {
        Some(score)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_csi_color_codes() {
        let input = "\x1B[32mcreated \x1B[39m changes/foo/proposal.md";
        assert_eq!(
            strip_ansi_escapes(input),
            "created  changes/foo/proposal.md"
        );
    }

    #[test]
    fn strips_tool_use_error_tags() {
        let input = "<tool_use_error>File has not been read yet.</tool_use_error>";
        assert_eq!(
            strip_tool_wrapper_tags(input),
            "File has not been read yet.",
        );
    }

    #[test]
    fn truncate_output_cleans_color_and_tags() {
        let raw = "\x1B[32mcreated \x1B[39m changes/foo/proposal.md";
        assert_eq!(
            truncate_output(raw),
            vec!["created  changes/foo/proposal.md".to_string()],
        );

        let raw = "<tool_use_error>File has not been read yet.</tool_use_error>";
        assert_eq!(
            truncate_output(raw),
            vec!["File has not been read yet.".to_string()],
        );
    }
}
