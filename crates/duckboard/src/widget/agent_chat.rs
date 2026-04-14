//! Agent chat widget — single block-aware text editor with input area.

use iced::widget::{column, container, row, text, text_editor, Space};

pub const INPUT_ID: &str = "agent-chat-input";
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
    /// Action from the chat text editor.
    ChatAction(text_edit::EditorAction),
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
pub fn build_chat_blocks(session: &ChatSession) -> Vec<Block> {
    let mut blocks = Vec::new();

    for msg in &session.messages {
        let role_label = match msg.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::System => "System",
        };

        for block in &msg.content {
            match block {
                ContentBlock::Text(t) => {
                    let kind = match msg.role {
                        Role::User => BlockKind::User,
                        Role::Assistant => BlockKind::Assistant,
                        Role::System => BlockKind::System,
                    };
                    let lines: Vec<String> = t.lines().map(String::from).collect();
                    blocks.push(Block {
                        kind,
                        label: role_label.to_string(),
                        lines,
                    });
                }
                ContentBlock::ToolUse { name, input, .. } => {
                    // Single-line summary of the tool call.
                    let summary = format_tool_summary(name, input);
                    blocks.push(Block {
                        kind: BlockKind::ToolUse,
                        label: summary,
                        lines: Vec::new(),
                    });
                }
                ContentBlock::ToolResult { name, output, .. } => {
                    let label = if name.is_empty() {
                        "✓ done".to_string()
                    } else {
                        format!("✓ {name}")
                    };
                    const MAX_LINES: usize = 10;
                    let all_lines: Vec<String> =
                        output.lines().map(String::from).collect();
                    let mut lines = if all_lines.len() > MAX_LINES {
                        let mut truncated = all_lines[..MAX_LINES].to_vec();
                        truncated.push(format!(
                            "… ({} more lines)",
                            all_lines.len() - MAX_LINES
                        ));
                        truncated
                    } else {
                        all_lines
                    };
                    // Skip entirely empty output.
                    if lines.len() == 1 && lines[0].is_empty() {
                        lines.clear();
                    }
                    blocks.push(Block {
                        kind: BlockKind::ToolResult,
                        label,
                        lines,
                    });
                }
            }
        }
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

pub fn view<'a>(
    _session: &'a ChatSession,
    chat_editor: &'a EditorState,
    input_value: &'a text_editor::Content,
    commands: &'a [SlashCommand],
    completion: &CompletionState,
    status: StatusInfo,
) -> Element<'a, Msg> {
    // Chat content — single block-aware text editor.
    let chat_view = text_edit::TextEdit::new(chat_editor, Msg::ChatAction)
        .show_gutter(false)
        .word_wrap(true)
        .read_only(true);

    let chat_scroll = container(chat_view)
        .width(Length::Fill)
        .height(Length::Fill);

    // Completion popup — always present in the tree to keep input_row at a
    // stable index (prevents iced from losing text_editor focus).
    let completion_el: Element<'a, Msg> = if completion.visible {
        let input_text = input_value.text();
        let query = input_text.trim_start_matches('/');
        let filtered = filter_commands(commands, query);
        if filtered.is_empty() {
            Space::new().height(0).into()
        } else {
            view_completion(commands, &filtered, completion.selected)
        }
    } else {
        Space::new().height(0).into()
    };

    // Input area.
    let input = text_editor(input_value)
        .on_action(Msg::EditorAction)
        .size(theme::FONT_MD)
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
        text(status_text).size(theme::FONT_XS).color(status_color),
    ]
    .spacing(theme::SPACING_SM)
    .align_y(iced::Alignment::Center);

    if status.is_streaming && status.esc_count < 2 {
        let hint = match status.esc_count {
            0 => "esc esc to cancel",
            _ => "esc to cancel",
        };
        left = left.push(
            text(hint).size(theme::FONT_XS).color(theme::text_muted()),
        );
    }

    // Right side: model + context.
    let ctx_pct = if status.context_max > 0 {
        (status.context_tokens as f32 / status.context_max as f32 * 100.0) as usize
    } else {
        0
    };
    let right = row![
        text(status.model).size(theme::FONT_XS).color(theme::text_muted()),
        text(format!("{}%", ctx_pct)).size(theme::FONT_XS).color(theme::text_muted()),
    ]
    .spacing(theme::SPACING_SM)
    .align_y(iced::Alignment::Center);

    container(
        row![left, Space::new().width(Length::Fill), right]
            .align_y(iced::Alignment::Center),
    )
    .padding([theme::SPACING_XS, theme::SPACING_SM])
    .style(status_bar_style)
    .into()
}

// ── Completion popup ────────────────────────────────────────────────────────

fn view_completion<'a>(
    commands: &'a [SlashCommand],
    filtered: &[(usize, i32)],
    selected: usize,
) -> Element<'a, Msg> {
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
                .size(theme::FONT_MD)
                .color(if is_selected { theme::text_primary() } else { theme::accent() }),
            Space::new().width(theme::SPACING_SM),
            text(&cmd.description)
                .size(theme::FONT_MD)
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
    container(items)
        .width(Length::Fill)
        .style(completion_style)
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

fn completion_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(theme::bg_elevated())),
        border: iced::Border {
            color: theme::border_color(),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn status_bar_style(_theme: &iced::Theme) -> container::Style {
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
