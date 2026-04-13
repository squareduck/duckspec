//! Dashboard area — stylish start screen with quick actions and project overview.

use iced::widget::{Space, button, column, container, row, scrollable, svg, text};
use iced::{Center, Element, Length};

use crate::data::ProjectData;
use crate::theme;
use crate::vcs::{ChangedFile, FileStatus};

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct State {}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    ChangeClicked(String),
    ArchivedChangeClicked(String),
    FindFile,
    GoToCaps,
    GoToCodex,
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(_state: &mut State, _message: Message) {
    // Navigation messages are handled by the parent (main.rs).
}

// ── Logo ────────────────────────────────────────────────────────────────────

const LOGO_SVG: &[u8] = include_bytes!("../../assets/logo.svg");

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(
    _state: &'a State,
    project: &'a ProjectData,
    changed_files: &'a [ChangedFile],
) -> Element<'a, Message> {
    let left = view_left(project);
    let right = view_right(project, changed_files);

    let content = row![
        container(left).width(Length::FillPortion(1)),
        Space::new().width(theme::SPACING_XL),
        container(right).width(Length::FillPortion(1)),
    ]
    .width(Length::Fill);

    scrollable(
        container(column![Space::new().height(40.0), content,].width(Length::Fill))
            .padding([theme::SPACING_XL, 48.0])
            .width(Length::Fill)
            .max_width(960),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}

// ── Left column: logo + quick actions ────────────────────────────────────────

fn view_left<'a>(project: &'a ProjectData) -> Element<'a, Message> {
    let duck_icon = svg(svg::Handle::from_memory(LOGO_SVG))
        .width(56.0)
        .height(56.0);

    let wordmark = column![
        row![
            text("duck").size(28.0).color(theme::TEXT_PRIMARY),
            text("spec").size(28.0).color(theme::ACCENT),
        ],
        text("specify \u{00b7} build \u{00b7} ship")
            .size(theme::FONT_SM)
            .color(theme::TEXT_MUTED),
    ]
    .spacing(2.0);

    let logo = row![duck_icon, wordmark]
        .spacing(theme::SPACING_MD)
        .align_y(Center);

    let mut actions = column![].spacing(2.0);

    actions = actions.push(action_row(
        "\u{1f50d}",
        "Find File",
        "Ctrl+P",
        theme::ACCENT,
        Message::FindFile,
    ));
    actions = actions.push(action_row(
        "\u{25c8}",
        "Capabilities",
        "S",
        theme::TEAL,
        Message::GoToCaps,
    ));
    actions = actions.push(action_row(
        "\u{2261}",
        "Codex",
        "X",
        theme::MAUVE,
        Message::GoToCodex,
    ));

    // Active changes as clickable actions.
    if !project.active_changes.is_empty() {
        actions = actions.push(Space::new().height(theme::SPACING_MD));
        actions = actions.push(
            text("Active Changes")
                .size(theme::FONT_SM)
                .color(theme::TEXT_MUTED),
        );
        actions = actions.push(Space::new().height(theme::SPACING_XS));
        for change in &project.active_changes {
            let step_count = change.steps.len();
            let detail = if step_count > 0 {
                format!("{} steps", step_count)
            } else {
                "no steps".to_string()
            };
            actions = actions.push(change_row(
                &change.name,
                &detail,
                theme::SUCCESS,
                Message::ChangeClicked(change.name.clone()),
            ));
        }
    }

    // Archived changes.
    if !project.archived_changes.is_empty() {
        actions = actions.push(Space::new().height(theme::SPACING_MD));
        actions = actions.push(
            text("Archived")
                .size(theme::FONT_SM)
                .color(theme::TEXT_MUTED),
        );
        actions = actions.push(Space::new().height(theme::SPACING_XS));
        for change in &project.archived_changes {
            actions = actions.push(change_row(
                &change.name,
                "",
                theme::TEXT_MUTED,
                Message::ArchivedChangeClicked(change.name.clone()),
            ));
        }
    }

    column![logo, Space::new().height(32.0), actions,]
        .width(Length::Fill)
        .into()
}

// ── Right column: stats + git status ─────────────────────────────────────────

fn view_right<'a>(
    project: &'a ProjectData,
    changed_files: &'a [ChangedFile],
) -> Element<'a, Message> {
    let mut col = column![].spacing(theme::SPACING_LG);

    // Stats bar.
    let stats = row![
        stat_pill(
            "\u{25c6}",
            &project.cap_count.to_string(),
            "caps",
            theme::TEAL
        ),
        stat_pill(
            "\u{2261}",
            &project.codex_count.to_string(),
            "codex",
            theme::MAUVE
        ),
        stat_pill(
            "\u{25b6}",
            &project.active_changes.len().to_string(),
            "changes",
            theme::PEACH
        ),
    ]
    .spacing(theme::SPACING_MD);

    col = col.push(stats);

    // Git status.
    col = col.push(Space::new().height(theme::SPACING_SM));
    col = col.push(section_header("\u{2387}", "Git Status", theme::PINK));

    if changed_files.is_empty() {
        col = col.push(
            container(
                text("Working tree clean")
                    .size(theme::FONT_MD)
                    .font(iced::Font::MONOSPACE)
                    .color(theme::TEXT_MUTED),
            )
            .padding([0.0, theme::SPACING_LG]),
        );
    } else {
        let mut git_col = column![].spacing(1.0);
        for cf in changed_files {
            let (marker, color) = match cf.status {
                FileStatus::Modified => ("M ", theme::WARNING),
                FileStatus::Added => ("A ", theme::SUCCESS),
                FileStatus::Deleted => ("D ", theme::ERROR),
            };
            git_col = git_col.push(
                container(
                    row![
                        text(marker)
                            .size(theme::FONT_MD)
                            .font(iced::Font::MONOSPACE)
                            .color(color),
                        text(cf.path.display().to_string())
                            .size(theme::FONT_MD)
                            .font(iced::Font::MONOSPACE)
                            .color(theme::TEXT_SECONDARY),
                    ]
                    .spacing(theme::SPACING_XS),
                )
                .padding([1.0, theme::SPACING_LG]),
            );
        }
        col = col.push(git_col);
    }

    // Project info.
    if let Some(root) = &project.project_root {
        col = col.push(Space::new().height(theme::SPACING_SM));
        col = col.push(section_header("\u{2302}", "Project", theme::LAVENDER));
        col = col.push(
            container(
                text(root.display().to_string())
                    .size(theme::FONT_MD)
                    .font(iced::Font::MONOSPACE)
                    .color(theme::TEXT_SECONDARY),
            )
            .padding([0.0, theme::SPACING_LG]),
        );
    }

    col.width(Length::Fill).into()
}

// ── Components ───────────────────────────────────────────────────────────────

fn action_row<'a>(
    icon: &'a str,
    label: &'a str,
    shortcut: &'a str,
    color: iced::Color,
    msg: Message,
) -> Element<'a, Message> {
    button(
        row![
            text(icon).size(theme::FONT_MD).color(color),
            text(label).size(theme::FONT_MD).color(theme::TEXT_PRIMARY),
            Space::new().width(Length::Fill),
            text(shortcut)
                .size(theme::FONT_SM)
                .font(iced::Font::MONOSPACE)
                .color(theme::TEXT_MUTED),
        ]
        .spacing(theme::SPACING_SM)
        .align_y(Center)
        .width(Length::Fill),
    )
    .on_press(msg)
    .width(Length::Fill)
    .padding([theme::SPACING_SM, theme::SPACING_MD])
    .style(theme::list_item)
    .into()
}

fn change_row<'a>(
    name: &'a str,
    detail: &str,
    color: iced::Color,
    msg: Message,
) -> Element<'a, Message> {
    let detail_owned = detail.to_string();
    button(
        row![
            text("\u{25b8}").size(theme::FONT_SM).color(color),
            text(name).size(theme::FONT_MD).color(theme::TEXT_PRIMARY),
            Space::new().width(Length::Fill),
            text(detail_owned)
                .size(theme::FONT_SM)
                .color(theme::TEXT_MUTED),
        ]
        .spacing(theme::SPACING_SM)
        .align_y(Center)
        .width(Length::Fill),
    )
    .on_press(msg)
    .width(Length::Fill)
    .padding([theme::SPACING_XS, theme::SPACING_MD])
    .style(theme::list_item)
    .into()
}

fn stat_pill<'a>(
    icon: &'a str,
    value: &str,
    label: &str,
    color: iced::Color,
) -> Element<'a, Message> {
    let value_owned = value.to_string();
    let label_owned = label.to_string();
    container(
        row![
            text(icon).size(theme::FONT_SM).color(color),
            text(value_owned).size(theme::FONT_MD).color(color),
            text(label_owned)
                .size(theme::FONT_SM)
                .color(theme::TEXT_MUTED),
        ]
        .spacing(theme::SPACING_XS)
        .align_y(Center),
    )
    .padding([theme::SPACING_SM, theme::SPACING_MD])
    .style(theme::elevated)
    .into()
}

fn section_header<'a, M: 'a>(icon: &'a str, label: &'a str, color: iced::Color) -> Element<'a, M> {
    row![
        text(icon).size(theme::FONT_MD).color(color),
        text(label).size(theme::FONT_MD).color(color),
    ]
    .spacing(theme::SPACING_SM)
    .into()
}
