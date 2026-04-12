//! Structural view widget for parsed duckspec artifacts.
//!
//! Renders specs, documents, and steps as navigable structured views
//! rather than raw markdown text.

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Center, Element, Length};

use duckpond::artifact::doc::{Document, Section};
use duckpond::artifact::spec::{Scenario, Spec, TestMarkerKind};
use duckpond::artifact::step::{PrerequisiteKind, Step, TaskContent};

use crate::theme;

// ── Parsed artifact enum ──────��────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum StructuralData {
    Spec(Spec),
    Document(Document),
    Step(Step),
}

// ── Messages ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum StructMsg {
    /// A backlink path was clicked (e.g. "tests/auth_test.rs:42").
    BacklinkClicked(String),
}

// ── View ───────────────────────────────────────────────────────────────────

pub fn view<'a>(data: &'a StructuralData) -> Element<'a, StructMsg> {
    let content: Element<'a, StructMsg> = match data {
        StructuralData::Spec(spec) => view_spec(spec),
        StructuralData::Document(doc) => view_document(doc),
        StructuralData::Step(step) => view_step(step),
    };

    scrollable(
        container(content)
            .padding(theme::SPACING_LG)
            .width(Length::Fill),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}

// ── Spec view ─────────────────────────────────────���────────────────────────

fn view_spec<'a>(spec: &'a Spec) -> Element<'a, StructMsg> {
    let mut col = column![].spacing(theme::SPACING_MD);

    // Title
    col = col.push(
        text(&spec.title)
            .size(18)
            .color(theme::STRUCTURAL_HEADING),
    );

    // Summary
    col = col.push(
        text(&spec.summary)
            .size(13)
            .color(theme::TEXT_SECONDARY),
    );

    // Requirements
    for req in &spec.requirements {
        col = col.push(Space::new().height(theme::SPACING_SM));
        col = col.push(view_requirement(req));
    }

    col.into()
}

fn view_requirement<'a>(
    req: &'a duckpond::artifact::spec::Requirement,
) -> Element<'a, StructMsg> {
    let mut col = column![].spacing(theme::SPACING_SM);

    // Requirement heading
    col = col.push(
        text(format!("Requirement: {}", req.name))
            .size(15)
            .color(theme::STRUCTURAL_HEADING),
    );

    // Test marker on the requirement itself
    if let Some(marker) = &req.test_marker {
        col = col.push(view_test_marker(marker));
    }

    // Scenarios
    for scenario in &req.scenarios {
        col = col.push(
            container(view_scenario(scenario))
                .padding([theme::SPACING_SM, theme::SPACING_LG]),
        );
    }

    container(col)
        .padding([theme::SPACING_SM, 0.0])
        .width(Length::Fill)
        .into()
}

fn view_scenario<'a>(scenario: &'a Scenario) -> Element<'a, StructMsg> {
    let mut col = column![].spacing(theme::SPACING_XS);

    // Scenario heading
    col = col.push(
        text(format!("Scenario: {}", scenario.name))
            .size(13)
            .color(theme::STRUCTURAL_SCENARIO),
    );

    // GWT clauses
    for clause in &scenario.givens {
        col = col.push(clause_row("GIVEN", &clause.text, theme::STRUCTURAL_CLAUSE_GIVEN));
    }
    for clause in &scenario.whens {
        col = col.push(clause_row("WHEN", &clause.text, theme::STRUCTURAL_CLAUSE_WHEN));
    }
    for clause in &scenario.thens {
        col = col.push(clause_row("THEN", &clause.text, theme::STRUCTURAL_CLAUSE_THEN));
    }

    // Test marker with backlinks
    if let Some(marker) = &scenario.test_marker {
        col = col.push(view_test_marker(marker));
    }

    col.into()
}

fn clause_row<'a, M: 'a>(
    keyword: &'a str,
    body: &'a str,
    color: iced::Color,
) -> Element<'a, M> {
    row![
        text(keyword)
            .size(11)
            .font(iced::Font::MONOSPACE)
            .color(color)
            .width(50.0),
        text(body).size(12).color(theme::TEXT_PRIMARY),
    ]
    .spacing(theme::SPACING_SM)
    .padding([0.0, theme::SPACING_LG])
    .align_y(Center)
    .into()
}

fn view_test_marker<'a>(
    marker: &'a duckpond::artifact::spec::TestMarker,
) -> Element<'a, StructMsg> {
    match &marker.kind {
        TestMarkerKind::Code { backlinks } => {
            let mut col = column![].spacing(2.0);
            col = col.push(
                text("test: code")
                    .size(11)
                    .font(iced::Font::MONOSPACE)
                    .color(theme::STRUCTURAL_MARKER),
            );
            for link in backlinks {
                col = col.push(
                    button(
                        text(&link.path)
                            .size(11)
                            .font(iced::Font::MONOSPACE),
                    )
                    .on_press(StructMsg::BacklinkClicked(link.path.clone()))
                    .padding([0.0, theme::SPACING_XS])
                    .style(theme::backlink_button),
                );
            }
            container(col)
                .padding([2.0, theme::SPACING_LG])
                .into()
        }
        TestMarkerKind::Manual { reason } => container(
            text(format!("manual: {reason}"))
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(theme::STRUCTURAL_MARKER),
        )
        .padding([2.0, theme::SPACING_LG])
        .into(),
        TestMarkerKind::Skip { reason } => container(
            text(format!("skip: {reason}"))
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(theme::STRUCTURAL_MARKER),
        )
        .padding([2.0, theme::SPACING_LG])
        .into(),
    }
}

// ── Document view ─────��────────────────────────────────────────────────────

fn view_document<'a>(doc: &'a Document) -> Element<'a, StructMsg> {
    let mut col = column![].spacing(theme::SPACING_MD);

    col = col.push(
        text(&doc.title)
            .size(18)
            .color(theme::STRUCTURAL_HEADING),
    );

    col = col.push(
        text(&doc.summary)
            .size(13)
            .color(theme::TEXT_SECONDARY),
    );

    for section in &doc.sections {
        col = col.push(view_section(section, 0));
    }

    col.into()
}

fn view_section<'a>(section: &'a Section, depth: usize) -> Element<'a, StructMsg> {
    let mut col = column![].spacing(theme::SPACING_XS);

    let size = match depth {
        0 => 15.0,
        1 => 14.0,
        _ => 13.0,
    };

    col = col.push(
        text(&section.heading)
            .size(size)
            .color(theme::STRUCTURAL_HEADING),
    );

    // Render body elements as text
    for elem in &section.body {
        if let Some(t) = element_text(elem) {
            col = col.push(text(t).size(12).color(theme::TEXT_PRIMARY));
        }
    }

    for child in &section.children {
        col = col.push(
            container(view_section(child, depth + 1))
                .padding([0.0, theme::SPACING_LG]),
        );
    }

    container(col)
        .padding([theme::SPACING_SM, 0.0])
        .width(Length::Fill)
        .into()
}

// ── Step view ────────────���─────────────────────────────────────────────────

fn view_step<'a>(step: &'a Step) -> Element<'a, StructMsg> {
    let mut col = column![].spacing(theme::SPACING_MD);

    col = col.push(
        text(&step.title)
            .size(18)
            .color(theme::STRUCTURAL_HEADING),
    );

    col = col.push(
        text(&step.summary)
            .size(13)
            .color(theme::TEXT_SECONDARY),
    );

    // Prerequisites
    if let Some(prereqs) = &step.prerequisites {
        if !prereqs.is_empty() {
            let mut prereq_col = column![
                text("Prerequisites").size(14).color(theme::STRUCTURAL_HEADING)
            ]
            .spacing(theme::SPACING_XS);
            for p in prereqs {
                let check = if p.checked { "\u{2611}" } else { "\u{2610}" };
                let label = match &p.kind {
                    PrerequisiteKind::StepRef { slug } => format!("@step {slug}"),
                    PrerequisiteKind::Freeform { text } => text.clone(),
                };
                let color = if p.checked {
                    theme::STRUCTURAL_TASK_DONE
                } else {
                    theme::TEXT_PRIMARY
                };
                prereq_col = prereq_col.push(
                    text(format!("{check} {label}"))
                        .size(12)
                        .color(color),
                );
            }
            col = col.push(prereq_col);
        }
    }

    // Tasks
    if !step.tasks.is_empty() {
        let mut task_col = column![
            text("Tasks").size(14).color(theme::STRUCTURAL_HEADING)
        ]
        .spacing(theme::SPACING_XS);
        for (i, task) in step.tasks.iter().enumerate() {
            let check = if task.checked { "\u{2611}" } else { "\u{2610}" };
            let label = match &task.content {
                TaskContent::Freeform { text } => text.clone(),
                TaskContent::SpecRef {
                    capability,
                    requirement,
                    scenario,
                } => format!("@spec {capability} {requirement}: {scenario}"),
            };
            let color = if task.checked {
                theme::STRUCTURAL_TASK_DONE
            } else {
                theme::TEXT_PRIMARY
            };
            task_col = task_col.push(
                text(format!("{check} {}. {label}", i + 1))
                    .size(12)
                    .color(color),
            );
            // Subtasks
            for sub in &task.subtasks {
                let sub_check = if sub.checked { "\u{2611}" } else { "\u{2610}" };
                let sub_label = match &sub.content {
                    TaskContent::Freeform { text } => text.clone(),
                    TaskContent::SpecRef {
                        capability,
                        requirement,
                        scenario,
                    } => format!("@spec {capability} {requirement}: {scenario}"),
                };
                let sub_color = if sub.checked {
                    theme::STRUCTURAL_TASK_DONE
                } else {
                    theme::TEXT_PRIMARY
                };
                task_col = task_col.push(
                    container(
                        text(format!("{sub_check} {sub_label}"))
                            .size(12)
                            .color(sub_color),
                    )
                    .padding([0.0, theme::SPACING_LG]),
                );
            }
        }
        col = col.push(task_col);
    }

    col.into()
}

// ── Helpers ──────────────────────────────────────────��─────────────────────

fn element_text(elem: &duckpond::parse::Element) -> Option<String> {
    match elem {
        duckpond::parse::Element::Block { content, .. } => Some(content.clone()),
        duckpond::parse::Element::ListItem { content, .. } => {
            Some(format!("\u{2022} {content}"))
        }
        duckpond::parse::Element::BlockQuoteItem { content, .. } => {
            Some(format!("> {content}"))
        }
        _ => None,
    }
}
