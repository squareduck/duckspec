//! Agent chat widget — scrollable message list with text input.

use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Element, Length};

use crate::chat_store::{ChatSession, ContentBlock, Role};
use crate::theme;

// ── Messages ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Msg {
    InputChanged(String),
    SendPressed,
    CancelPressed,
}

// ── View ────────────────────────────────────────────────────────────────────

pub fn view<'a>(session: &'a ChatSession, input_value: &str) -> Element<'a, Msg> {
    // Header.
    let status_text = if session.is_streaming {
        "streaming…"
    } else {
        "connected"
    };
    let header = container(
        row![
            text("Agent Chat").size(theme::FONT_MD).color(theme::TEXT_PRIMARY),
            Space::new().width(Length::Fill),
            text(status_text).size(theme::FONT_SM).color(theme::TEXT_MUTED),
        ]
        .spacing(theme::SPACING_SM)
        .align_y(iced::Alignment::Center),
    )
    .padding(theme::SPACING_SM)
    .style(header_style);

    // Message list.
    let mut messages = column![].spacing(theme::SPACING_SM).padding(theme::SPACING_SM);

    for msg in &session.messages {
        messages = messages.push(view_message(msg));
    }

    // Show streaming partial text.
    if session.is_streaming && !session.pending_text.is_empty() {
        messages = messages.push(
            container(
                text(&session.pending_text)
                    .size(theme::FONT_MD)
                    .font(iced::Font::MONOSPACE)
                    .color(theme::TEXT_PRIMARY),
            )
            .padding(theme::SPACING_SM)
            .width(Length::Fill)
            .style(assistant_bubble),
        );
    }

    let message_scroll = scrollable(messages)
        .width(Length::Fill)
        .height(Length::Fill);

    // Input area.
    let input = text_input("Send a message…", input_value)
        .on_input(Msg::InputChanged)
        .on_submit(Msg::SendPressed)
        .size(theme::FONT_MD)
        .id("agent-chat-input");

    let action_button = if session.is_streaming {
        button(text("Cancel").size(theme::FONT_SM).color(theme::ERROR))
            .on_press(Msg::CancelPressed)
            .style(theme::list_item)
    } else {
        button(text("Send").size(theme::FONT_SM).color(theme::ACCENT))
            .on_press(Msg::SendPressed)
            .style(theme::list_item)
    };

    let input_row = container(
        row![input, action_button]
            .spacing(theme::SPACING_SM)
            .align_y(iced::Alignment::Center),
    )
    .padding(theme::SPACING_SM)
    .style(header_style);

    column![header, message_scroll, input_row]
        .height(Length::Fill)
        .into()
}

fn view_message<'a>(msg: &'a crate::chat_store::ChatMessage) -> Element<'a, Msg> {
    let mut blocks = column![].spacing(theme::SPACING_XS);

    for block in &msg.content {
        match block {
            ContentBlock::Text(t) => {
                let color = match msg.role {
                    Role::User => theme::TEXT_PRIMARY,
                    Role::Assistant => theme::TEXT_PRIMARY,
                    Role::System => theme::TEXT_MUTED,
                };
                blocks = blocks.push(
                    text(t.as_str())
                        .size(theme::FONT_MD)
                        .font(iced::Font::MONOSPACE)
                        .color(color),
                );
            }
            ContentBlock::ToolUse { name, input, .. } => {
                blocks = blocks.push(
                    text(format!("⚙ {name}"))
                        .size(theme::FONT_SM)
                        .color(theme::ACCENT_DIM),
                );
                if !input.is_empty() {
                    blocks = blocks.push(
                        text(truncate(input, 200))
                            .size(theme::FONT_SM)
                            .font(iced::Font::MONOSPACE)
                            .color(theme::TEXT_MUTED),
                    );
                }
            }
            ContentBlock::ToolResult { name, output, .. } => {
                let label = if name.is_empty() {
                    "result".to_string()
                } else {
                    format!("{name} result")
                };
                blocks = blocks.push(
                    text(format!("✓ {label}"))
                        .size(theme::FONT_SM)
                        .color(theme::SUCCESS),
                );
                if !output.is_empty() {
                    blocks = blocks.push(
                        text(truncate(output, 200))
                            .size(theme::FONT_SM)
                            .font(iced::Font::MONOSPACE)
                            .color(theme::TEXT_MUTED),
                    );
                }
            }
        }
    }

    let bubble_style = match msg.role {
        Role::User => user_bubble,
        Role::Assistant | Role::System => assistant_bubble,
    };

    container(blocks)
        .padding(theme::SPACING_SM)
        .width(Length::Fill)
        .style(bubble_style)
        .into()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

// ── Styles ──────────────────────────────────────────────────────────────────

fn header_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(theme::BG_SURFACE)),
        border: iced::Border {
            color: theme::BORDER_COLOR,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn user_bubble(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(theme::BG_ELEVATED)),
        border: iced::Border {
            color: theme::ACCENT,
            width: 1.0,
            radius: theme::BORDER_RADIUS.into(),
        },
        ..Default::default()
    }
}

fn assistant_bubble(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(theme::BG_SURFACE)),
        border: iced::Border {
            color: theme::BORDER_COLOR,
            width: 1.0,
            radius: theme::BORDER_RADIUS.into(),
        },
        ..Default::default()
    }
}
