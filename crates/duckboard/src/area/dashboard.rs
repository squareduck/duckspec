//! Dashboard area — sleek start screen with changes and explorations.

use iced::widget::{button, column, container, row, scrollable, svg, text, Space};
use iced::widget::text::Wrapping;
use iced::{Center, Element, Length};

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
    ExplorationClicked(String),
    AddExploration,
    ShowAudit,
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(_state: &mut State, _message: Message) {
    // Navigation messages are handled by the parent (main.rs).
}

// ── Icons ───────────────────────────────────────────────────────────────────

const ICON_BRANCH: &[u8] = include_bytes!("../../assets/icon_branch.svg");
const ICON_EXPLORE: &[u8] = include_bytes!("../../assets/icon_explore.svg");

const ICON_SIZE: f32 = 14.0;

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(
    _state: &'a State,
    project: &'a ProjectData,
    explorations: &'a [String],
) -> Element<'a, Message> {
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

    let mut content = column![logo, Space::new().height(theme::SPACING_XL)].spacing(0.0);

    // ── Audit summary ───────────────────────────────────────────────────────
    let total_errors: usize = project.validations.values().map(|v| v.total_count()).sum();
    if total_errors > 0 {
        let change_count = project.validations.len();
        let summary_text = format!(
            "{} error{} across {} change{}",
            total_errors,
            if total_errors == 1 { "" } else { "s" },
            change_count,
            if change_count == 1 { "" } else { "s" },
        );
        let audit_card = container(
            row![
                column![
                    text("Audit")
                        .size(theme::font_md())
                        .color(theme::text_primary()),
                    text(summary_text)
                        .size(theme::font_sm())
                        .color(theme::error()),
                ]
                .spacing(theme::SPACING_XS),
                Space::new().width(Length::Fill),
                button(
                    text("View")
                        .size(theme::font_md())
                        .color(theme::accent()),
                )
                .on_press(Message::ShowAudit)
                .padding([theme::SPACING_XS, theme::SPACING_MD])
                .style(theme::dashboard_action),
            ]
            .align_y(Center)
            .width(Length::Fill),
        )
        .padding(theme::SPACING_MD)
        .width(Length::Fill)
        .style(theme::audit_card);

        content = content.push(audit_card);
        content = content.push(Space::new().height(theme::SPACING_XL));
    } else if !project.active_changes.is_empty() {
        let audit_card = container(
            row![
                text("Audit")
                    .size(theme::font_md())
                    .color(theme::text_primary()),
                Space::new().width(Length::Fill),
                text("No errors")
                    .size(theme::font_sm())
                    .color(theme::success()),
            ]
            .align_y(Center)
            .width(Length::Fill),
        )
        .padding(theme::SPACING_MD)
        .width(Length::Fill)
        .style(theme::audit_card);

        content = content.push(audit_card);
        content = content.push(Space::new().height(theme::SPACING_XL));
    }

    // ── Explorations ────────────────────────────────────────────────────────
    let has_explorations = !explorations.is_empty();
    if has_explorations {
        content = content.push(
            text("Explorations")
                .size(theme::font_sm())
                .color(theme::text_muted()),
        );
        content = content.push(Space::new().height(theme::SPACING_SM));

        let mut list = column![].spacing(2.0);
        for name in explorations {
            list = list.push(exploration_row(name));
        }
        content = content.push(list);
        content = content.push(Space::new().height(theme::SPACING_LG));
    }

    // ── New Exploration button ───────────────────────────────────────────────
    content = content.push(
        button(
            row![
                text("+").size(theme::font_md()).color(theme::accent()),
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
        .width(Length::Fill),
    );
    content = content.push(Space::new().height(theme::SPACING_XL));

    // ── Active changes ──────────────────────────────────────────────────────
    if !project.active_changes.is_empty() {
        content = content.push(
            text("Active Changes")
                .size(theme::font_sm())
                .color(theme::text_muted()),
        );
        content = content.push(Space::new().height(theme::SPACING_SM));

        let mut list = column![].spacing(2.0);
        for change in &project.active_changes {
            let step_count = change.steps.len();
            let detail = if step_count > 0 {
                format!("{} steps", step_count)
            } else {
                String::new()
            };
            list = list.push(change_row(
                &change.name,
                &detail,
                Message::ChangeClicked(change.name.clone()),
            ));
        }
        content = content.push(list);
        content = content.push(Space::new().height(theme::SPACING_LG));
    }

    // ── Archived changes ────────────────────────────────────────────────────
    if !project.archived_changes.is_empty() {
        content = content.push(
            text("Archived")
                .size(theme::font_sm())
                .color(theme::text_muted()),
        );
        content = content.push(Space::new().height(theme::SPACING_SM));

        let mut list = column![].spacing(2.0);
        for change in &project.archived_changes {
            list = list.push(change_row(
                &change.name,
                "",
                Message::ArchivedChangeClicked(change.name.clone()),
            ));
        }
        content = content.push(list);
    }

    // ── Empty state ─────────────────────────────────────────────────────────
    if project.active_changes.is_empty()
        && project.archived_changes.is_empty()
        && !has_explorations
    {
        content = content.push(
            container(
                text("No changes yet. Start an exploration or create a change in duckspec.")
                    .size(theme::font_md())
                    .color(theme::text_muted()),
            )
            .padding([theme::SPACING_LG, 0.0]),
        );
    }

    scrollable(
        container(content.width(Length::Fill))
            .padding([theme::SPACING_XL, 48.0])
            .width(Length::Fill)
            .max_width(520)
            .center_x(Length::Fill),
    )
    .direction(theme::thin_scrollbar_direction())
    .style(theme::thin_scrollbar)
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}

// ── Components ──────────────────────────────────────────────────────────────

fn change_row<'a>(
    name: &'a str,
    detail: &str,
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

    if !detail_owned.is_empty() {
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
