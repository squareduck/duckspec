//! Dashboard area — project overview and navigation hub.

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Element, Length};

use crate::data::ProjectData;
use crate::theme;

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct State {}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    ChangeClicked(String),
    ArchivedChangeClicked(String),
    NewChange,
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(_state: &mut State, _message: Message) {
    // Navigation messages are handled by the parent (main.rs).
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let _ = state;

    let header = text("Dashboard").size(20).color(theme::TEXT_PRIMARY);

    let stats = row![
        stat_card("Capabilities", project.cap_count.to_string()),
        stat_card("Codex entries", project.codex_count.to_string()),
        stat_card("Active changes", project.active_changes.len().to_string()),
    ]
    .spacing(theme::SPACING_MD);

    let active_header = text("Active changes")
        .size(14)
        .color(theme::TEXT_SECONDARY);

    let mut active_list = column![].spacing(theme::SPACING_XS);
    if project.active_changes.is_empty() {
        active_list = active_list.push(
            text("No active changes")
                .size(13)
                .color(theme::TEXT_MUTED),
        );
    } else {
        for change in &project.active_changes {
            active_list = active_list.push(change_card(change, false));
        }
    }

    let new_change_btn = button(
        row![
            text("+").size(16).color(theme::ACCENT),
            text("New change").size(13).color(theme::TEXT_PRIMARY),
        ]
        .spacing(theme::SPACING_SM),
    )
    .on_press(Message::NewChange)
    .padding([theme::SPACING_SM, theme::SPACING_MD])
    .style(theme::list_item);

    let archive_header = text("Archived changes")
        .size(14)
        .color(theme::TEXT_SECONDARY);

    let mut archive_list = column![].spacing(theme::SPACING_XS);
    if project.archived_changes.is_empty() {
        archive_list = archive_list.push(
            text("No archived changes")
                .size(13)
                .color(theme::TEXT_MUTED),
        );
    } else {
        for change in &project.archived_changes {
            archive_list = archive_list.push(change_card(change, true));
        }
    }

    let content = column![
        header,
        Space::new().height(theme::SPACING_MD),
        stats,
        Space::new().height(theme::SPACING_XL),
        active_header,
        Space::new().height(theme::SPACING_SM),
        active_list,
        Space::new().height(theme::SPACING_SM),
        new_change_btn,
        Space::new().height(theme::SPACING_XL),
        archive_header,
        Space::new().height(theme::SPACING_SM),
        archive_list,
    ]
    .spacing(0.0)
    .width(Length::Fill);

    scrollable(
        container(content)
            .padding(theme::SPACING_XL)
            .width(Length::Fill)
            .max_width(800),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}

fn stat_card<'a>(label: &str, value: String) -> Element<'a, Message> {
    container(
        column![
            text(value).size(24).color(theme::ACCENT),
            text(label.to_string()).size(12).color(theme::TEXT_SECONDARY),
        ]
        .spacing(theme::SPACING_XS),
    )
    .padding(theme::SPACING_LG)
    .style(theme::elevated)
    .into()
}

fn change_card<'a>(change: &'a crate::data::ChangeData, archived: bool) -> Element<'a, Message> {
    let step_info = if change.steps.is_empty() {
        String::new()
    } else {
        format!(" \u{2022} {} steps", change.steps.len())
    };

    let mut badges = row![].spacing(theme::SPACING_SM);
    if change.has_proposal {
        badges = badges.push(text("proposal").size(11).color(theme::TEXT_MUTED));
    }
    if change.has_design {
        badges = badges.push(text("design").size(11).color(theme::TEXT_MUTED));
    }
    if !change.cap_tree.is_empty() {
        let caps_label = format!("{} caps", change.cap_tree.len());
        badges = badges.push(text(caps_label).size(11).color(theme::TEXT_MUTED));
    }

    let msg = if archived {
        Message::ArchivedChangeClicked(change.name.clone())
    } else {
        Message::ChangeClicked(change.name.clone())
    };

    let name = change.name.clone();
    button(
        column![
            row![
                text(name).size(14).color(theme::TEXT_PRIMARY),
                text(step_info).size(12).color(theme::TEXT_SECONDARY),
            ]
            .spacing(theme::SPACING_SM),
            badges,
        ]
        .spacing(theme::SPACING_XS),
    )
    .on_press(msg)
    .width(Length::Fill)
    .padding([theme::SPACING_SM, theme::SPACING_MD])
    .style(theme::list_item)
    .into()
}
