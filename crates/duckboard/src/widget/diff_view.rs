//! Inline diff view widget.
//!
//! Renders a unified diff with line numbers, colored backgrounds for
//! added/removed lines, and hunk headers. Structured to support future
//! syntax highlighting (each line's text is isolated for span replacement).

use iced::widget::{column, container, row, scrollable, text, Space};
use iced::{Element, Length};

use crate::theme;
use crate::vcs::{DiffData, DiffLine, FileStatus, LineKind};

const LINENO_WIDTH: f32 = 48.0;
const SIGN_WIDTH: f32 = 16.0;
const FONT_SIZE: f32 = 13.0;
const LINE_HEIGHT: f32 = 20.0;

/// Render a full inline diff view.
pub fn view<'a, M: 'a>(diff: &'a DiffData) -> Element<'a, M> {
    let mut col = column![].spacing(0.0);

    // File header.
    col = col.push(file_header(diff));

    if diff.hunks.is_empty() {
        let msg = match diff.status {
            FileStatus::Added => "New file (empty)",
            FileStatus::Deleted => "File deleted (empty)",
            FileStatus::Modified => "No visible changes",
        };
        col = col.push(
            container(text(msg).size(FONT_SIZE).color(theme::TEXT_MUTED))
                .padding(theme::SPACING_LG),
        );
    }

    for hunk in &diff.hunks {
        col = col.push(hunk_header(&hunk.header));
        for line in &hunk.lines {
            col = col.push(diff_line(line));
        }
    }

    scrollable(col)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

fn file_header<'a, M: 'a>(diff: &'a DiffData) -> Element<'a, M> {
    let status_label = match diff.status {
        FileStatus::Modified => "modified",
        FileStatus::Added => "added",
        FileStatus::Deleted => "deleted",
    };
    let status_color = theme::vcs_status_color(&diff.status);

    container(
        row![
            text(diff.path.display().to_string())
                .size(13)
                .font(iced::Font::MONOSPACE)
                .color(theme::TEXT_PRIMARY),
            Space::new().width(theme::SPACING_SM),
            text(status_label).size(11).color(status_color),
        ]
        .spacing(theme::SPACING_SM)
        .align_y(iced::Center),
    )
    .padding([theme::SPACING_SM, theme::SPACING_LG])
    .width(Length::Fill)
    .style(theme::elevated)
    .into()
}

fn hunk_header<'a, M: 'a>(header: &'a str) -> Element<'a, M> {
    container(
        text(header.trim_end())
            .size(12)
            .font(iced::Font::MONOSPACE)
            .color(theme::TEXT_MUTED),
    )
    .padding([2.0, theme::SPACING_LG])
    .width(Length::Fill)
    .style(theme::diff_hunk_header)
    .into()
}

fn diff_line<'a, M: 'a>(line: &'a DiffLine) -> Element<'a, M> {
    let old_no = lineno_text(line.old_lineno);
    let new_no = lineno_text(line.new_lineno);

    let sign = match line.kind {
        LineKind::Added => "+",
        LineKind::Removed => "-",
        LineKind::Context => " ",
    };

    let sign_color = match line.kind {
        LineKind::Added => theme::SUCCESS,
        LineKind::Removed => theme::ERROR,
        LineKind::Context => theme::TEXT_MUTED,
    };

    let text_color = match line.kind {
        LineKind::Added => theme::TEXT_PRIMARY,
        LineKind::Removed => theme::TEXT_PRIMARY,
        LineKind::Context => theme::TEXT_SECONDARY,
    };

    let line_text = line.text.trim_end_matches('\n');

    let row_content = row![
        text(old_no)
            .size(11)
            .font(iced::Font::MONOSPACE)
            .color(theme::TEXT_MUTED)
            .width(LINENO_WIDTH / 2.0),
        text(new_no)
            .size(11)
            .font(iced::Font::MONOSPACE)
            .color(theme::TEXT_MUTED)
            .width(LINENO_WIDTH / 2.0),
        text(sign)
            .size(FONT_SIZE)
            .font(iced::Font::MONOSPACE)
            .color(sign_color)
            .width(SIGN_WIDTH),
        text(line_text)
            .size(FONT_SIZE)
            .font(iced::Font::MONOSPACE)
            .color(text_color),
    ]
    .spacing(0.0)
    .height(LINE_HEIGHT)
    .align_y(iced::Center);

    let style: fn(&iced::Theme) -> container::Style = match line.kind {
        LineKind::Added => theme::diff_added,
        LineKind::Removed => theme::diff_removed,
        LineKind::Context => transparent_style,
    };

    container(row_content)
        .padding([0.0, theme::SPACING_SM])
        .width(Length::Fill)
        .style(style)
        .into()
}

fn lineno_text(n: Option<u32>) -> String {
    match n {
        Some(n) => format!("{n:>3}"),
        None => "   ".to_string(),
    }
}

fn transparent_style(_theme: &iced::Theme) -> container::Style {
    container::Style::default()
}
