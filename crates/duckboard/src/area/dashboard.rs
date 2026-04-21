//! Dashboard area — sleek start screen with changes and audit overview.

use iced::widget::{button, column, container, row, scrollable, svg, text, Space};
use iced::widget::text::Wrapping;
use iced::{Center, Element, Length};

use crate::data::{ChangeValidation, ProjectData};
use crate::theme;

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct State {}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    ChangeClicked(String),
    ArchivedChangeClicked(String),
    ExplorationClicked(String),
    AddExploration,
    RefreshAudit,
    SelectAuditError { change: String, artifact_id: String },
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(_state: &mut State, _message: Message) {
    // Navigation messages are handled by the parent (main.rs).
}

// ── Icons ───────────────────────────────────────────────────────────────────

const ICON_BRANCH: &[u8] = include_bytes!("../../assets/icon_branch.svg");
const ICON_EXPLORE: &[u8] = include_bytes!("../../assets/icon_explore.svg");

const ICON_SIZE: f32 = 14.0;
const SECTION_HEADING_SIZE: f32 = 18.0;

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(
    _state: &'a State,
    project: &'a ProjectData,
    explorations: &'a [String],
) -> Element<'a, Message> {
    let header = view_header();

    let items = view_items_panel(project, explorations);
    let audit = view_audit_panel(project);

    let divider = container(Space::new().height(Length::Fill))
        .width(1.0)
        .style(theme::divider);

    let panels = row![
        container(items)
            .width(Length::FillPortion(1))
            .height(Length::Fill)
            .padding([0.0, theme::SPACING_XL]),
        divider,
        container(audit)
            .width(Length::FillPortion(1))
            .height(Length::Fill)
            .padding([0.0, theme::SPACING_XL]),
    ]
    .height(Length::Fill);

    let body = column![
        header,
        Space::new().height(theme::SPACING_LG),
        panels,
    ]
    .height(Length::Fill);

    container(body)
        .padding([theme::SPACING_XL, theme::SPACING_LG])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── Header ──────────────────────────────────────────────────────────────────

fn view_header<'a>() -> Element<'a, Message> {
    let sep = || text(" \u{00b7} ").size(theme::font_sm()).color(theme::text_muted());
    let muted = |s| text(s).size(theme::font_sm()).color(theme::text_muted());

    let logo = column![
        row![
            text("duck").size(22.0).color(theme::text_primary()),
            text("spec").size(22.0).color(theme::accent()),
        ],
        row![
            muted("explore"), sep(),
            muted("propose"), sep(),
            muted("design"), sep(),
            text("doc").size(theme::font_sm()).color(theme::text_primary()),
            text(" ").size(theme::font_sm()),
            text("spec").size(theme::font_sm()).color(theme::accent()),
            sep(),
            muted("step"), sep(),
            muted("apply"), sep(),
            muted("archive"),
        ],
    ]
    .spacing(1.0);

    container(logo)
        .padding([0.0, theme::SPACING_LG])
        .into()
}

// ── Items panel (left) ─────────────────────────────────────────────────────

fn view_items_panel<'a>(
    project: &'a ProjectData,
    explorations: &'a [String],
) -> Element<'a, Message> {
    let mut content = column![].spacing(theme::SPACING_LG);

    // ── Active Changes ──────────────────────────────────────────────────
    if !project.active_changes.is_empty() {
        let mut list = column![].spacing(2.0);
        for change in &project.active_changes {
            let step_count = change.steps.len();
            let detail = if step_count > 0 {
                format!("{} step{}", step_count, if step_count == 1 { "" } else { "s" })
            } else {
                String::new()
            };
            let error_count = project
                .validations
                .get(&change.name)
                .map(|v| v.total_count())
                .unwrap_or(0);
            list = list.push(change_row(
                &change.name,
                &detail,
                error_count,
                Message::ChangeClicked(change.name.clone()),
            ));
        }
        content = content.push(section("Changes", list.into()));
    }

    // ── Explorations ────────────────────────────────────────────────────
    let mut exp_list = column![].spacing(2.0);
    for name in explorations {
        exp_list = exp_list.push(exploration_row(name));
    }
    let plus_icon = svg(svg::Handle::from_memory(crate::widget::collapsible::ICON_PLUS))
        .width(theme::font_md())
        .height(theme::font_md())
        .style(theme::svg_tint(theme::accent()));
    let new_btn = button(
        row![
            plus_icon,
            text("New Exploration")
                .size(theme::font_md())
                .color(theme::accent()),
        ]
        .spacing(theme::SPACING_XS)
        .align_y(Center),
    )
    .on_press(Message::AddExploration)
    .padding([theme::SPACING_SM, theme::SPACING_MD])
    .style(theme::dashboard_action)
    .width(Length::Fill);
    exp_list = exp_list.push(Space::new().height(theme::SPACING_SM));
    exp_list = exp_list.push(new_btn);
    content = content.push(section("Explorations", exp_list.into()));

    // ── Archived ────────────────────────────────────────────────────────
    if !project.archived_changes.is_empty() {
        let mut list = column![].spacing(2.0);
        for change in &project.archived_changes {
            list = list.push(change_row(
                &change.name,
                "",
                0,
                Message::ArchivedChangeClicked(change.name.clone()),
            ));
        }
        content = content.push(section("Archived", list.into()));
    }

    scrollable(content.width(Length::Fill))
        .direction(theme::thin_scrollbar_direction())
        .style(theme::thin_scrollbar)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

// ── Audit panel (right) ────────────────────────────────────────────────────

fn view_audit_panel<'a>(project: &'a ProjectData) -> Element<'a, Message> {
    let total_errors: usize = project.validations.values().map(|v| v.total_count()).sum();
    let change_count_with_errors = project
        .validations
        .values()
        .filter(|v| v.total_count() > 0)
        .count();

    // ── Header ─────────────────────────────────────────────────────────
    let summary: Element<'a, Message> = if project.active_changes.is_empty() {
        text("No active changes to audit")
            .size(theme::font_sm())
            .color(theme::text_muted())
            .into()
    } else if total_errors == 0 {
        text("All checks passed")
            .size(theme::font_sm())
            .color(theme::success())
            .into()
    } else {
        text(format!(
            "{} error{} across {} change{}",
            total_errors,
            if total_errors == 1 { "" } else { "s" },
            change_count_with_errors,
            if change_count_with_errors == 1 { "" } else { "s" },
        ))
        .size(theme::font_sm())
        .color(theme::error())
        .into()
    };

    let refresh_btn: Element<'a, Message> = if project.active_changes.is_empty() {
        Space::new().into()
    } else {
        button(
            text("Refresh")
                .size(theme::font_sm())
                .color(theme::accent()),
        )
        .on_press(Message::RefreshAudit)
        .padding([theme::SPACING_XS, theme::SPACING_MD])
        .style(theme::dashboard_action)
        .into()
    };

    let header = row![
        column![
            text("Audit")
                .size(SECTION_HEADING_SIZE)
                .color(theme::text_primary()),
            summary,
        ]
        .spacing(2.0),
        Space::new().width(Length::Fill),
        refresh_btn,
    ]
    .align_y(Center)
    .width(Length::Fill);

    let header_section = container(header)
        .padding([0.0, theme::SPACING_SM])
        .width(Length::Fill);

    // ── Error cards ────────────────────────────────────────────────────
    let mut content = column![header_section].spacing(theme::SPACING_MD);

    if total_errors > 0 {
        let mut changes: Vec<_> = project
            .validations
            .iter()
            .filter(|(_, v)| v.total_count() > 0)
            .collect();
        changes.sort_by_key(|(name, _)| name.as_str());

        for (change_name, validation) in changes {
            content = content.push(view_audit_card(change_name, validation));
        }
    }

    scrollable(content.width(Length::Fill))
        .direction(theme::thin_scrollbar_direction())
        .style(theme::thin_scrollbar)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

/// Render a single change's errors as a card.
fn view_audit_card<'a>(
    change_name: &'a str,
    validation: &'a ChangeValidation,
) -> Element<'a, Message> {
    let icon = svg(svg::Handle::from_memory(ICON_BRANCH))
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .style(theme::svg_tint(theme::text_muted()));
    let card_header = row![
        icon,
        text(change_name)
            .size(theme::font_md())
            .color(theme::text_primary())
            .wrapping(Wrapping::None),
        Space::new().width(Length::Fill),
        text(format!(
            "{} error{}",
            validation.total_count(),
            if validation.total_count() == 1 { "" } else { "s" }
        ))
        .size(theme::font_sm())
        .color(theme::text_muted()),
    ]
    .spacing(theme::SPACING_SM)
    .align_y(Center);

    let mut card = column![card_header].spacing(theme::SPACING_SM);

    // Per-file error groups.
    for (path, errors) in &validation.file_errors {
        let artifact_id = path.clone();
        let change = change_name.to_string();
        let file_link = button(
            text(path.as_str())
                .size(theme::font_sm())
                .color(theme::accent()),
        )
        .on_press(Message::SelectAuditError {
            change,
            artifact_id,
        })
        .padding(0.0)
        .style(theme::link_button);

        let mut error_list = column![].spacing(2.0);
        for err in errors {
            error_list = error_list.push(
                text(err.as_str())
                    .size(theme::font_sm())
                    .color(theme::error()),
            );
        }

        let group = column![
            file_link,
            container(error_list).padding([0.0, theme::SPACING_MD]),
        ]
        .spacing(2.0);

        card = card.push(group);
    }

    // Cross-file change-level errors.
    if !validation.change_errors.is_empty() {
        let mut structural = column![
            text("Structural")
                .size(theme::font_sm())
                .color(theme::text_secondary()),
        ]
        .spacing(2.0);

        for err in &validation.change_errors {
            structural = structural.push(
                container(
                    text(err.as_str())
                        .size(theme::font_sm())
                        .color(theme::error()),
                )
                .padding([0.0, theme::SPACING_MD]),
            );
        }
        card = card.push(structural);
    }

    container(card)
        .padding(theme::SPACING_MD)
        .width(Length::Fill)
        .style(theme::audit_card)
        .into()
}

// ── Components ──────────────────────────────────────────────────────────────

fn section<'a>(title: &'a str, body: Element<'a, Message>) -> Element<'a, Message> {
    column![
        container(
            text(title)
                .size(SECTION_HEADING_SIZE)
                .color(theme::text_primary()),
        )
        .padding([0.0, theme::SPACING_SM]),
        Space::new().height(theme::SPACING_SM),
        body,
    ]
    .spacing(0.0)
    .into()
}

fn change_row<'a>(
    name: &'a str,
    detail: &str,
    error_count: usize,
    msg: Message,
) -> Element<'a, Message> {
    let detail_owned = detail.to_string();
    let icon = svg(svg::Handle::from_memory(ICON_BRANCH))
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .style(theme::svg_tint(theme::text_muted()));

    let mut label = row![
        icon,
        text(name)
            .size(theme::font_md())
            .color(theme::text_primary())
            .wrapping(Wrapping::None),
    ]
    .spacing(theme::SPACING_SM)
    .align_y(Center)
    .width(Length::Fill);

    if error_count > 0 {
        label = label.push(Space::new().width(Length::Fill));
        label = label.push(
            text(error_count.to_string())
                .size(theme::font_sm())
                .color(theme::error()),
        );
    } else if !detail_owned.is_empty() {
        label = label.push(Space::new().width(Length::Fill));
        label = label.push(
            text(detail_owned)
                .size(theme::font_sm())
                .color(theme::text_muted()),
        );
    }

    button(label)
        .on_press(msg)
        .width(Length::Fill)
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .style(theme::list_item)
        .into()
}

fn exploration_row(name: &str) -> Element<'_, Message> {
    let icon = svg(svg::Handle::from_memory(ICON_EXPLORE))
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .style(theme::svg_tint(theme::text_muted()));

    let label = row![
        icon,
        text(name)
            .size(theme::font_md())
            .color(theme::text_primary())
            .wrapping(Wrapping::None),
    ]
    .spacing(theme::SPACING_SM)
    .align_y(Center)
    .width(Length::Fill);

    button(label)
        .on_press(Message::ExplorationClicked(name.to_string()))
        .width(Length::Fill)
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .style(theme::list_item)
        .into()
}

// ── Breadcrumbs ──────────────────────────────────────────────────────────────

pub fn breadcrumbs() -> Vec<String> {
    vec!["Dashboard".into()]
}
