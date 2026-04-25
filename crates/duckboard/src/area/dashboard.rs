//! Dashboard area — sleek start screen with changes and audit overview.

use std::path::{Path, PathBuf};

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, scrollable, svg, text};
use iced::{Center, Element, Length};

use crate::data::{ChangeValidation, ProjectAudit, ProjectData};
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
    SelectAuditError { change: String, artifact_id: String },
    /// Show the project-picker modal.
    OpenProjectPicker,
    /// Open a specific project root immediately (used by the recents list).
    OpenRecent(PathBuf),
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
    explorations: &'a [crate::chat_store::Exploration],
    recent_projects: &'a [PathBuf],
) -> Element<'a, Message> {
    let header = view_header(project);

    let body: Element<'a, Message> = if project.project_root.is_none() {
        column![
            header,
            Space::new().height(theme::SPACING_XL),
            view_empty_state(recent_projects),
        ]
        .height(Length::Fill)
        .into()
    } else {
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

        column![header, Space::new().height(theme::SPACING_LG), panels]
            .height(Length::Fill)
            .into()
    };

    container(body)
        .padding([theme::SPACING_XL, theme::SPACING_LG])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── Header ──────────────────────────────────────────────────────────────────

fn view_header<'a>(project: &'a ProjectData) -> Element<'a, Message> {
    let sep = || {
        text(" \u{00b7} ")
            .size(theme::font_sm())
            .color(theme::text_muted())
    };
    let muted = |s| text(s).size(theme::font_sm()).color(theme::text_muted());

    let logo = column![
        row![
            text("duck").size(22.0).color(theme::text_primary()),
            text("spec").size(22.0).color(theme::accent()),
        ],
        row![
            muted("explore"),
            sep(),
            muted("propose"),
            sep(),
            muted("design"),
            sep(),
            text("doc")
                .size(theme::font_sm())
                .color(theme::text_primary()),
            text(" ").size(theme::font_sm()),
            text("spec").size(theme::font_sm()).color(theme::accent()),
            sep(),
            muted("step"),
            sep(),
            muted("apply"),
            sep(),
            muted("archive"),
        ],
    ]
    .spacing(1.0);

    // Mirror the column layout of the panels below (equal FillPortion(1)
    // halves separated by a 1px spacer, each padded by SPACING_XL, and
    // each inner section offset by SPACING_SM) so the project name lines
    // up with the "Audit" heading, and the logo sits above "Explorations".
    let section_pad = |el: Element<'a, Message>| -> Element<'a, Message> {
        container(el)
            .padding([0.0, theme::SPACING_SM])
            .width(Length::Fill)
            .into()
    };

    let left_half = container(section_pad(logo.into()))
        .width(Length::FillPortion(1))
        .padding([0.0, theme::SPACING_XL]);

    let right_half: Element<'a, Message> = if let Some(root) = &project.project_root {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root.display().to_string());
        let content = row![
            text(name)
                .size(22.0)
                .color(theme::text_primary())
                .wrapping(Wrapping::None),
            Space::new().width(Length::Fill),
            open_project_button("Change project"),
        ]
        .align_y(Center)
        .width(Length::Fill);
        container(section_pad(content.into()))
            .width(Length::FillPortion(1))
            .padding([0.0, theme::SPACING_XL])
            .into()
    } else {
        container(Space::new())
            .width(Length::FillPortion(1))
            .padding([0.0, theme::SPACING_XL])
            .into()
    };

    // 1px spacer where the panels' divider would be, so the two halves
    // split at the exact same x as the divider below.
    let spacer = Space::new().width(1.0);

    row![left_half, spacer, right_half]
        .align_y(Center)
        .width(Length::Fill)
        .into()
}

fn open_project_button<'a>(label: &'a str) -> Element<'a, Message> {
    let plus_icon = svg(svg::Handle::from_memory(
        crate::widget::collapsible::ICON_PLUS,
    ))
    .width(theme::font_md())
    .height(theme::font_md())
    .style(theme::svg_tint(theme::accent()));
    button(
        row![
            plus_icon,
            text(label).size(theme::font_md()).color(theme::accent()),
        ]
        .spacing(theme::SPACING_XS)
        .align_y(Center),
    )
    .on_press(Message::OpenProjectPicker)
    .padding([theme::SPACING_SM, theme::SPACING_MD])
    .style(theme::dashboard_action)
    .into()
}

// ── Empty state (no project open) ──────────────────────────────────────────

fn view_empty_state<'a>(recent: &'a [PathBuf]) -> Element<'a, Message> {
    let prompt = text("No project open")
        .size(theme::font_md())
        .color(theme::text_secondary());
    let hint = text("Open a directory to get started. \u{2318}O")
        .size(theme::font_sm())
        .color(theme::text_muted());

    let mut col = column![
        prompt,
        hint,
        Space::new().height(theme::SPACING_MD),
        open_project_button("Open project..."),
    ]
    .spacing(theme::SPACING_SM)
    .align_x(iced::Alignment::Start)
    .max_width(520.0);

    if !recent.is_empty() {
        col = col.push(Space::new().height(theme::SPACING_LG));
        col = col.push(
            text("Recent")
                .size(theme::font_sm())
                .color(theme::text_secondary()),
        );
        for path in recent {
            col = col.push(recent_row(path));
        }
    }

    // Mirror the header's nested padding (XL from the column container
    // plus SM from the section wrapper) so empty-state text starts at
    // the exact same x as the "duckspec" title above it.
    container(container(col).padding([0.0, theme::SPACING_SM]))
        .padding([0.0, theme::SPACING_XL])
        .width(Length::Fill)
        .into()
}

fn recent_row<'a>(path: &'a Path) -> Element<'a, Message> {
    let label = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());
    let full = path.display().to_string();

    let content = column![
        text(label)
            .size(theme::font_md())
            .color(theme::text_primary())
            .wrapping(Wrapping::None),
        text(full)
            .size(theme::font_sm())
            .color(theme::text_muted())
            .wrapping(Wrapping::None),
    ]
    .spacing(2.0);

    button(content)
        .on_press(Message::OpenRecent(path.to_path_buf()))
        .width(Length::Fill)
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .style(theme::list_item)
        .into()
}

// ── Items panel (left) ─────────────────────────────────────────────────────

fn view_items_panel<'a>(
    project: &'a ProjectData,
    explorations: &'a [crate::chat_store::Exploration],
) -> Element<'a, Message> {
    let mut content = column![].spacing(theme::SPACING_LG);

    // ── Active Changes ──────────────────────────────────────────────────
    if !project.active_changes.is_empty() {
        let mut list = column![].spacing(2.0);
        for change in &project.active_changes {
            let step_count = change.steps.len();
            let detail = if step_count > 0 {
                format!(
                    "{} step{}",
                    step_count,
                    if step_count == 1 { "" } else { "s" }
                )
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
    // Explorations owned by an idea are hidden here; they surface on the
    // Ideas list instead.
    let mut exp_list = column![].spacing(2.0);
    for exp in explorations.iter().filter(|e| e.idea_path.is_none()) {
        exp_list = exp_list.push(exploration_row(&exp.id, &exp.display_name));
    }
    let plus_icon = svg(svg::Handle::from_memory(
        crate::widget::collapsible::ICON_PLUS,
    ))
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
    let change_error_total: usize = project.validations.values().map(|v| v.total_count()).sum();
    let project_audit_total = project.project_audit.total_count();
    let total_errors = change_error_total + project_audit_total;
    let change_count_with_errors = project
        .validations
        .values()
        .filter(|v| v.total_count() > 0)
        .count();

    // ── Header ─────────────────────────────────────────────────────────
    let summary: Element<'a, Message> = if total_errors == 0 {
        text("All checks passed")
            .size(theme::font_sm())
            .color(theme::success())
            .into()
    } else if change_count_with_errors == 0 {
        text(format!(
            "{} project-level error{}",
            total_errors,
            if total_errors == 1 { "" } else { "s" },
        ))
        .size(theme::font_sm())
        .color(theme::error())
        .into()
    } else {
        text(format!(
            "{} error{} across {} change{}",
            total_errors,
            if total_errors == 1 { "" } else { "s" },
            change_count_with_errors,
            if change_count_with_errors == 1 {
                ""
            } else {
                "s"
            },
        ))
        .size(theme::font_sm())
        .color(theme::error())
        .into()
    };

    let header = column![
        text("Audit")
            .size(SECTION_HEADING_SIZE)
            .color(theme::text_primary()),
        summary,
    ]
    .spacing(2.0)
    .width(Length::Fill);

    let header_section = container(header)
        .padding([0.0, theme::SPACING_SM])
        .width(Length::Fill);

    // ── Error cards ────────────────────────────────────────────────────
    let mut content = column![header_section].spacing(theme::SPACING_MD);

    // Project-level audit card (artifact errors, backlinks, coverage, etc.)
    if !project.project_audit.is_empty() {
        content = content.push(view_project_audit_card(&project.project_audit));
    }

    // Per-change cards.
    let mut changes: Vec<_> = project
        .validations
        .iter()
        .filter(|(_, v)| v.total_count() > 0)
        .collect();
    changes.sort_by_key(|(name, _)| name.as_str());

    for (change_name, validation) in changes {
        content = content.push(view_audit_card(change_name, validation));
    }

    // Right padding keeps card borders clear of the overlaid scrollbar,
    // matching the left gap between the divider and the card.
    let padded = container(content.width(Length::Fill))
        .padding(iced::Padding {
            top: 0.0,
            right: theme::SPACING_XL,
            bottom: 0.0,
            left: 0.0,
        })
        .width(Length::Fill);

    scrollable(padded)
        .direction(theme::thin_scrollbar_direction())
        .style(theme::thin_scrollbar)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

/// Render the project-level audit card with one section per finding kind.
fn view_project_audit_card<'a>(audit: &'a ProjectAudit) -> Element<'a, Message> {
    let icon = svg(svg::Handle::from_memory(ICON_BRANCH))
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .style(theme::svg_tint(theme::text_muted()));
    let total = audit.total_count();
    let header = row![
        icon,
        text("Project")
            .size(theme::font_md())
            .color(theme::text_primary())
            .wrapping(Wrapping::None),
        Space::new().width(Length::Fill),
        text(format!(
            "{} error{}",
            total,
            if total == 1 { "" } else { "s" }
        ))
        .size(theme::font_sm())
        .color(theme::text_muted()),
    ]
    .spacing(theme::SPACING_SM)
    .align_y(Center);

    let mut card = column![header].spacing(theme::SPACING_SM);

    // Section: artifact errors (per-file, grouped by path).
    if !audit.artifact_errors.is_empty() {
        card = card.push(section_header("Artifacts"));
        for (path, errs) in &audit.artifact_errors {
            let mut body = column![
                text(path.as_str())
                    .size(theme::font_sm())
                    .color(theme::text_secondary()),
            ]
            .spacing(2.0);
            for err in errs {
                body = body.push(
                    container(
                        text(err.as_str())
                            .size(theme::font_sm())
                            .color(theme::error()),
                    )
                    .padding([0.0, theme::SPACING_MD]),
                );
            }
            card = card.push(body);
        }
    }

    // Section: unresolved backlinks.
    if !audit.unresolved_backlinks.is_empty() {
        card = card.push(section_header("Unresolved backlinks"));
        for bl in &audit.unresolved_backlinks {
            card = card.push(
                container(
                    column![
                        text(format!("{}:{}", bl.source_path, bl.line))
                            .size(theme::font_sm())
                            .color(theme::text_secondary()),
                        text(bl.scenario_display.as_str())
                            .size(theme::font_sm())
                            .color(theme::error()),
                    ]
                    .spacing(2.0),
                )
                .padding([0.0, theme::SPACING_MD]),
            );
        }
    }

    // Section: test:code scenarios with no source backlink.
    if !audit.missing_backlink_scenarios.is_empty() {
        card = card.push(section_header("Scenarios missing backlinks"));
        for key in &audit.missing_backlink_scenarios {
            card = card.push(
                container(
                    text(key.as_str())
                        .size(theme::font_sm())
                        .color(theme::error()),
                )
                .padding([0.0, theme::SPACING_MD]),
            );
        }
    }

    // Section: test:code scenarios not covered by step tasks, grouped per change.
    if !audit.missing_step_coverage.is_empty() {
        card = card.push(section_header("Missing step coverage"));
        for (change_name, keys) in &audit.missing_step_coverage {
            let mut body = column![
                text(change_name.as_str())
                    .size(theme::font_sm())
                    .color(theme::text_secondary()),
            ]
            .spacing(2.0);
            for key in keys {
                body = body.push(
                    container(
                        text(key.as_str())
                            .size(theme::font_sm())
                            .color(theme::error()),
                    )
                    .padding([0.0, theme::SPACING_MD]),
                );
            }
            card = card.push(body);
        }
    }

    // Section: unresolved step @spec refs.
    if !audit.unresolved_step_refs.is_empty() {
        card = card.push(section_header("Unresolved step refs"));
        for (change_name, key) in &audit.unresolved_step_refs {
            card = card.push(
                container(
                    column![
                        text(change_name.as_str())
                            .size(theme::font_sm())
                            .color(theme::text_secondary()),
                        text(key.as_str())
                            .size(theme::font_sm())
                            .color(theme::error()),
                    ]
                    .spacing(2.0),
                )
                .padding([0.0, theme::SPACING_MD]),
            );
        }
    }

    container(card)
        .padding(theme::SPACING_MD)
        .width(Length::Fill)
        .style(theme::audit_card)
        .into()
}

fn section_header<'a>(title: &'a str) -> Element<'a, Message> {
    text(title)
        .size(theme::font_sm())
        .color(theme::text_secondary())
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
            if validation.total_count() == 1 {
                ""
            } else {
                "s"
            }
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

fn exploration_row<'a>(id: &str, display_name: &'a str) -> Element<'a, Message> {
    let icon = svg(svg::Handle::from_memory(ICON_EXPLORE))
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .style(theme::svg_tint(theme::text_muted()));

    let label = row![
        icon,
        text(display_name)
            .size(theme::font_md())
            .color(theme::text_primary())
            .wrapping(Wrapping::None),
    ]
    .spacing(theme::SPACING_SM)
    .align_y(Center)
    .width(Length::Fill);

    button(label)
        .on_press(Message::ExplorationClicked(id.to_string()))
        .width(Length::Fill)
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .style(theme::list_item)
        .into()
}

// ── Breadcrumbs ──────────────────────────────────────────────────────────────

pub fn breadcrumbs() -> Vec<String> {
    vec!["Dashboard".into()]
}
