//! Agent chat widget — per-message text editors in a scrollable column.

use iced::widget::{button, column, container, row, rule, scrollable, text, text_editor, Space};

pub const INPUT_ID: &str = "agent-chat-input";
pub const CHAT_SCROLLABLE_ID: &str = "agent-chat-scroll";
use iced::{Element, Length};

use crate::agent::SlashCommand;
use crate::chat_store::{ChatSession, ContentBlock, Role};
use crate::theme;
use crate::widget::text_edit::{self, Block, BlockKind, EditorState};

// ── Messages ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Msg {
    EditorAction(text_editor::Action),
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
                let result_lines = if let Some((_, ContentBlock::ToolResult { id: rid, output, .. })) =
                    items.get(i + 1)
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
        .map(|c| {
            if c == '\t' || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect()
}

/// Produce a short human-readable summary of a tool call.
/// E.g. `⚙ Read /src/main.rs` or `⚙ Edit /src/lib.rs`.
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
            return format!("⚙ {} {}", name, short);
        }

        let pattern = map.get("pattern").and_then(|v| v.as_str());
        if let Some(pat) = pattern {
            let truncated = if pat.len() > 40 { &pat[..40] } else { pat };
            return format!("⚙ {} \"{}\"", name, truncated);
        }

        let command = map.get("command").and_then(|v| v.as_str());
        if let Some(cmd) = command {
            let truncated = if cmd.len() > 50 { &cmd[..50] } else { cmd };
            return format!("⚙ {} `{}`", name, truncated);
        }
    }

    format!("⚙ {name}")
}

// ── View ────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    _session: &'a ChatSession,
    blocks: &'a [Block],
    editors: &'a [EditorState],
    collapsed: &'a [bool],
    input_value: &'a text_editor::Content,
    commands: &'a [SlashCommand],
    completion: &CompletionState,
    status: StatusInfo,
) -> Element<'a, Msg> {
    // Chat content — scrollable column of full-width sections.
    let mut chat_col = column![].spacing(0.0);

    for (i, block) in blocks.iter().enumerate() {
        let is_collapsed = collapsed.get(i).copied().unwrap_or(false);
        let block_el = view_block(i, block, editors.get(i), is_collapsed);
        chat_col = chat_col.push(block_el);
    }

    let chat_scroll = scrollable(chat_col)
        .direction(theme::thin_scrollbar_direction())
        .style(theme::thin_scrollbar)
        .width(Length::Fill)
        .height(Length::Fill)
        .anchor_bottom()
        .id(CHAT_SCROLLABLE_ID);

    // Completion popup — always rendered with the same widget type so iced's
    // tree diff preserves text_editor focus. When hidden, the inner column
    // is empty and the border is suppressed so the popup collapses cleanly.
    let has_completion = completion.visible && {
        let input_text = input_value.text();
        let query = input_text.trim_start_matches('/');
        !filter_commands(commands, query).is_empty()
    };
    let completion_col = if completion.visible {
        let input_text = input_value.text();
        let query = input_text.trim_start_matches('/');
        let filtered = filter_commands(commands, query);
        view_completion_col(commands, &filtered, completion.selected)
    } else {
        column![].spacing(0.0)
    };
    let completion_el: Element<'a, Msg> = container(completion_col)
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme| {
            if has_completion {
                container::Style {
                    background: Some(iced::Background::Color(theme::bg_elevated())),
                    border: iced::Border {
                        color: theme::border_color(),
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                }
            } else {
                container::Style::default()
            }
        })
        .into();

    // Input area.
    let input = text_editor(input_value)
        .on_action(Msg::EditorAction)
        .size(theme::font_md())
        .height(Length::Shrink)
        .id(INPUT_ID);

    let input_row = container(input)
        .padding(theme::SPACING_SM)
        .style(header_style);

    // Status bar below input.
    let status_bar = view_status_bar(status);

    column![chat_scroll, completion_el, input_row, status_bar]
        .height(Length::Fill)
        .into()
}

/// Render a single chat block as a full-width section followed by a subtle divider.
fn view_block<'a>(
    idx: usize,
    block: &'a Block,
    editor: Option<&'a EditorState>,
    collapsed: bool,
) -> Element<'a, Msg> {
    let bg = text_edit::block_kind_bg(block.kind);
    let header_color = block_header_color(block.kind);
    let has_content = !block.lines.is_empty();
    let is_tool = matches!(block.kind, BlockKind::ToolUse | BlockKind::ToolResult);

    let section_bg = move |_theme: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(bg)),
        ..Default::default()
    };

    let divider = rule::horizontal(1).style(move |_theme: &iced::Theme| rule::Style {
        color: theme::border_color(),
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    });

    // ── Tool blocks ─────────────────────────────────────────────────────
    if is_tool {
        let label = text(&block.label).size(theme::font_sm()).color(header_color);

        if !has_content {
            let header_container = container(label)
                .padding([theme::SPACING_SM, theme::SPACING_SM]);
            let section = container(header_container)
                .width(Length::Fill)
                .style(section_bg);
            return column![section, divider].into();
        }

        let arrow = text(if collapsed { "▸" } else { "▾" })
            .size(theme::font_sm())
            .color(header_color);
        let header_row = row![arrow, label]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Alignment::Center);
        let header_btn = button(header_row)
            .on_press(Msg::ToggleCollapse(idx))
            .padding(0.0)
            .style(|_theme, _status| iced::widget::button::Style {
                background: None,
                ..Default::default()
            });

        let header_container = container(header_btn)
            .padding([theme::SPACING_SM, theme::SPACING_SM]);

        let mut block_col = column![header_container];

        if !collapsed && let Some(ed) = editor {
            block_col = block_col.push(
                text_edit::TextEdit::new(ed, move |action| Msg::ChatAction(idx, action))
                    .show_gutter(false)
                    .word_wrap(true)
                    .read_only(true)
                    .fit_content(true),
            );
        }

        let section = container(block_col)
            .width(Length::Fill)
            .style(section_bg);
        return column![section, divider].into();
    }

    // ── User / Assistant / System: message sections ─────────────────────

    let arrow = text(if collapsed { "▸" } else { "▾" })
        .size(theme::font_sm())
        .color(header_color);
    let label = text(&block.label).size(theme::font_sm()).color(header_color);

    let header_row = row![arrow, label]
        .spacing(theme::SPACING_XS)
        .align_y(iced::Alignment::Center);

    let header_btn = button(header_row)
        .on_press(Msg::ToggleCollapse(idx))
        .padding(0.0)
        .style(|_theme, _status| iced::widget::button::Style {
            background: None,
            ..Default::default()
        });

    let header_container = container(header_btn)
        .padding([theme::SPACING_SM, theme::SPACING_SM]);

    let mut block_col = column![header_container];

    if has_content && !collapsed && let Some(ed) = editor {
        block_col = block_col.push(
            text_edit::TextEdit::new(ed, move |action| Msg::ChatAction(idx, action))
                .show_gutter(false)
                .word_wrap(true)
                .read_only(true)
                .fit_content(true),
        );
    }

    let section = container(block_col)
        .width(Length::Fill)
        .style(section_bg);

    column![section, divider].into()
}

/// Header label color for a block kind (re-exported from text_edit for convenience).
fn block_header_color(kind: BlockKind) -> iced::Color {
    match kind {
        BlockKind::User => theme::accent(),
        BlockKind::Assistant => theme::text_secondary(),
        BlockKind::ToolUse => theme::accent_dim(),
        BlockKind::ToolResult => theme::success(),
        BlockKind::System => theme::text_muted(),
    }
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

fn view_status_bar<'a>(status: StatusInfo) -> Element<'a, Msg> {
    // Left side: status + cancel hint.
    let (status_text, status_color) = if status.is_streaming && status.esc_count >= 2 {
        ("cancelling…", theme::error())
    } else if status.is_streaming {
        ("streaming", theme::accent_dim())
    } else {
        ("ready", theme::text_muted())
    };

    let mut left = row![
        text(status_text).size(theme::font_sm()).color(status_color),
    ]
    .spacing(theme::SPACING_SM)
    .align_y(iced::Alignment::Center);

    if status.is_streaming && status.esc_count < 2 {
        let hint = match status.esc_count {
            0 => "esc esc to cancel",
            _ => "esc to cancel",
        };
        left = left.push(
            text(hint).size(theme::font_sm()).color(theme::text_muted()),
        );
    }

    // Right side: model + context tokens + percentage.
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
    let right = row![
        text(status.model).size(theme::font_sm()).color(theme::text_muted()),
        text(format!(
            "{} / {} ({}%)",
            format_number(status.context_tokens),
            format_number(status.context_max),
            ctx_pct,
        ))
        .size(theme::font_sm())
        .color(ctx_color),
    ]
    .spacing(theme::SPACING_SM)
    .align_y(iced::Alignment::Center);

    let bar = container(
        row![left, Space::new().width(Length::Fill), right]
            .align_y(iced::Alignment::Center),
    )
    .padding([theme::SPACING_XS, theme::SPACING_SM])
    .style(status_bar_style);

    column![
        rule::horizontal(1).style(|_theme: &iced::Theme| rule::Style {
            color: theme::border_color(),
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        }),
        bar,
    ]
    .into()
}

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
        let bg = if is_selected {
            theme::bg_hover()
        } else {
            theme::bg_elevated()
        };
        let label = row![
            text(format!("/{}", cmd.name))
                .size(theme::font_md())
                .color(if is_selected { theme::text_primary() } else { theme::accent() }),
            Space::new().width(theme::SPACING_SM),
            text(&cmd.description)
                .size(theme::font_md())
                .color(if is_selected { theme::text_secondary() } else { theme::text_muted() }),
        ]
        .align_y(iced::Alignment::Center);
        items = items.push(
            container(label)
                .width(Length::Fill)
                .padding([theme::SPACING_XS, theme::SPACING_SM])
                .style(move |_theme: &iced::Theme| container::Style {
                    background: Some(iced::Background::Color(bg)),
                    ..Default::default()
                }),
        );
    }
    items
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

// ── Styles ──────────────────────────────────────────────────────────────────

fn header_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(theme::bg_surface())),
        border: iced::Border {
            color: theme::border_color(),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn status_bar_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(theme::bg_surface())),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_csi_color_codes() {
        let input = "\x1B[32mcreated \x1B[39m changes/foo/proposal.md";
        assert_eq!(strip_ansi_escapes(input), "created  changes/foo/proposal.md");
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
