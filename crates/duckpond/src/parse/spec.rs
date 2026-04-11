use crate::artifact::spec::*;
use crate::error::ParseError;
use crate::parse::{BlockKind, Element, Span};

/// Parse a sequence of L1 elements into a capability spec.
///
/// Returns all accumulated errors — the parser does not short-circuit on the
/// first problem.
pub fn parse_spec(elements: &[Element]) -> Result<Spec, Vec<ParseError>> {
    let mut errors = Vec::new();

    // -- H1 title -----------------------------------------------------------
    let (title, title_span, mut cursor) = match elements.first() {
        Some(Element::Heading {
            level: 1,
            content,
            span,
        }) => (content.clone(), *span, 1),
        Some(elem) => {
            errors.push(ParseError::ContentBeforeH1 { span: elem.span() });
            return Err(errors);
        }
        None => {
            errors.push(ParseError::MissingH1 {
                span: Span {
                    offset: 0,
                    length: 0,
                },
            });
            return Err(errors);
        }
    };

    // -- Summary paragraph ---------------------------------------------------
    let (summary, summary_span) = match elements.get(cursor) {
        Some(Element::Block {
            kind: BlockKind::Paragraph,
            content,
            span,
        }) => {
            cursor += 1;
            (content.clone(), *span)
        }
        Some(elem) => {
            errors.push(ParseError::MissingSummary { span: elem.span() });
            (String::new(), elem.span())
        }
        None => {
            errors.push(ParseError::MissingSummary { span: title_span });
            return Err(errors);
        }
    };

    // -- Description (blocks before first H2) --------------------------------
    let mut description = Vec::new();
    while cursor < elements.len() {
        match &elements[cursor] {
            Element::Heading { level: 2.., .. } => break,
            elem => {
                description.push(elem.clone());
                cursor += 1;
            }
        }
    }

    // -- Requirements --------------------------------------------------------
    let mut requirements = Vec::new();

    while cursor < elements.len() {
        match &elements[cursor] {
            Element::Heading {
                level: 2,
                content,
                span,
            } => {
                let (req, new_cursor) =
                    parse_requirement(elements, cursor, content, *span, &mut errors);
                requirements.push(req);
                cursor = new_cursor;
            }
            Element::Heading { level, span, .. } if *level >= 4 => {
                errors.push(ParseError::HeadingTooDeep { span: *span });
                cursor += 1;
            }
            _ => {
                cursor += 1;
            }
        }
    }

    if errors.is_empty() {
        Ok(Spec {
            title,
            title_span,
            summary,
            summary_span,
            description,
            requirements,
        })
    } else {
        Err(errors)
    }
}

fn parse_requirement(
    elements: &[Element],
    start: usize,
    heading: &str,
    heading_span: Span,
    errors: &mut Vec<ParseError>,
) -> (Requirement, usize) {
    let name = if let Some(name) = heading.strip_prefix("Requirement: ") {
        let name = name.trim().to_string();
        if name.contains(':') {
            errors.push(ParseError::RequirementNameColon { span: heading_span });
        }
        name
    } else {
        errors.push(ParseError::InvalidRequirementPrefix { span: heading_span });
        heading.to_string()
    };

    let mut cursor = start + 1;
    let mut prose = Vec::new();
    let mut test_marker = None;
    let mut scenarios = Vec::new();

    while cursor < elements.len() {
        match &elements[cursor] {
            Element::Heading { level: 1 | 2, .. } => break,

            Element::Heading {
                level: 3,
                content,
                span,
            } => {
                let (scenario, new_cursor) =
                    parse_scenario(elements, cursor, content, *span, errors);
                scenarios.push(scenario);
                cursor = new_cursor;
            }

            Element::Heading { level, span, .. } if *level >= 4 => {
                errors.push(ParseError::HeadingTooDeep { span: *span });
                cursor += 1;
            }

            Element::BlockQuoteItem { content, span } => {
                if let Some(marker) = try_parse_test_marker(content, *span) {
                    test_marker = Some(marker);
                } else {
                    errors.push(ParseError::InvalidTestMarker { span: *span });
                }
                cursor += 1;
                // Consume backlink lines after `test: code`.
                if matches!(
                    test_marker,
                    Some(TestMarker {
                        kind: TestMarkerKind::Code { .. },
                        ..
                    })
                ) {
                    let backlinks = consume_backlinks(elements, &mut cursor);
                    if let Some(TestMarker {
                        kind:
                            TestMarkerKind::Code {
                                backlinks: ref mut bl,
                            },
                        ..
                    }) = test_marker
                    {
                        *bl = backlinks;
                    }
                }
            }

            elem => {
                prose.push(elem.clone());
                cursor += 1;
            }
        }
    }

    // Validate: requirement must have prose or scenarios.
    if prose.is_empty() && scenarios.is_empty() {
        errors.push(ParseError::EmptyRequirement {
            name: name.clone(),
            span: heading_span,
        });
    }

    // Validate: test markers resolve for all scenarios.
    for scenario in &scenarios {
        if scenario.test_marker.is_none() && test_marker.is_none() {
            errors.push(ParseError::UnresolvedTestMarker {
                name: scenario.name.clone(),
                span: scenario.name_span,
            });
        }
    }

    (
        Requirement {
            name,
            name_span: heading_span,
            prose,
            test_marker,
            scenarios,
        },
        cursor,
    )
}

// ---------------------------------------------------------------------------
// GWT phase state machine
// ---------------------------------------------------------------------------

/// Tracks the current phase while parsing GWT clauses.
/// Valid transitions: Start → Given → When → Then
/// AND stays in the current phase.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GwtPhase {
    Start,
    Given,
    When,
    Then,
}

/// Which keyword was found on a list item.
enum GwtKeyword {
    Given,
    When,
    Then,
    And,
}

fn parse_scenario(
    elements: &[Element],
    start: usize,
    heading: &str,
    heading_span: Span,
    errors: &mut Vec<ParseError>,
) -> (Scenario, usize) {
    let name = if let Some(name) = heading.strip_prefix("Scenario: ") {
        name.trim().to_string()
    } else {
        errors.push(ParseError::InvalidScenarioPrefix { span: heading_span });
        heading.to_string()
    };

    let mut cursor = start + 1;
    let mut givens = Vec::new();
    let mut whens = Vec::new();
    let mut thens = Vec::new();
    let mut phase = GwtPhase::Start;
    let mut test_marker = None;
    let mut has_non_gwt = false;

    while cursor < elements.len() {
        match &elements[cursor] {
            Element::Heading { .. } => break,

            Element::ListItem { content, span, .. } => {
                match try_parse_gwt_keyword(content) {
                    Some((GwtKeyword::Given, text)) => {
                        if phase == GwtPhase::Start || phase == GwtPhase::Given {
                            phase = GwtPhase::Given;
                            givens.push(Clause { text, span: *span });
                        } else {
                            errors.push(ParseError::GwtClauseOutOfOrder { span: *span });
                        }
                    }
                    Some((GwtKeyword::When, text)) => {
                        if phase == GwtPhase::Start
                            || phase == GwtPhase::Given
                            || phase == GwtPhase::When
                        {
                            phase = GwtPhase::When;
                            whens.push(Clause { text, span: *span });
                        } else {
                            errors.push(ParseError::GwtClauseOutOfOrder { span: *span });
                        }
                    }
                    Some((GwtKeyword::Then, text)) => {
                        if phase == GwtPhase::When || phase == GwtPhase::Then {
                            phase = GwtPhase::Then;
                            thens.push(Clause { text, span: *span });
                        } else {
                            errors.push(ParseError::GwtClauseOutOfOrder { span: *span });
                        }
                    }
                    Some((GwtKeyword::And, text)) => match phase {
                        GwtPhase::Given => givens.push(Clause { text, span: *span }),
                        GwtPhase::When => whens.push(Clause { text, span: *span }),
                        GwtPhase::Then => thens.push(Clause { text, span: *span }),
                        GwtPhase::Start => {
                            errors.push(ParseError::GwtClauseOutOfOrder { span: *span });
                        }
                    },
                    None => {
                        errors.push(ParseError::InvalidGwtKeyword { span: *span });
                    }
                }
                cursor += 1;
            }

            Element::BlockQuoteItem { content, span } => {
                if let Some(marker) = try_parse_test_marker(content, *span) {
                    test_marker = Some(marker);
                } else {
                    errors.push(ParseError::InvalidTestMarker { span: *span });
                }
                cursor += 1;
                // Consume backlink lines after `test: code`.
                if matches!(
                    test_marker,
                    Some(TestMarker {
                        kind: TestMarkerKind::Code { .. },
                        ..
                    })
                ) {
                    let backlinks = consume_backlinks(elements, &mut cursor);
                    if let Some(TestMarker {
                        kind:
                            TestMarkerKind::Code {
                                backlinks: ref mut bl,
                            },
                        ..
                    }) = test_marker
                    {
                        *bl = backlinks;
                    }
                }
            }

            elem => {
                if !has_non_gwt {
                    errors.push(ParseError::UnexpectedScenarioContent { span: elem.span() });
                    has_non_gwt = true;
                }
                cursor += 1;
            }
        }
    }

    // Validate WHEN and THEN presence.
    if whens.is_empty() {
        errors.push(ParseError::MissingWhen {
            name: name.clone(),
            span: heading_span,
        });
    }
    if thens.is_empty() {
        errors.push(ParseError::MissingThen {
            name: name.clone(),
            span: heading_span,
        });
    }

    (
        Scenario {
            name,
            name_span: heading_span,
            givens,
            whens,
            thens,
            test_marker,
        },
        cursor,
    )
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Extract a GWT keyword and the trailing text from a list item content.
fn try_parse_gwt_keyword(content: &str) -> Option<(GwtKeyword, String)> {
    let trimmed = content.trim();
    if !trimmed.starts_with("**") {
        return None;
    }
    let after_stars = &trimmed[2..];
    let end = after_stars.find("**")?;
    let keyword_str = &after_stars[..end];
    let text = after_stars[end + 2..].trim().to_string();

    let keyword = match keyword_str {
        "GIVEN" => GwtKeyword::Given,
        "WHEN" => GwtKeyword::When,
        "THEN" => GwtKeyword::Then,
        "AND" => GwtKeyword::And,
        _ => return None,
    };

    Some((keyword, text))
}

fn try_parse_test_marker(content: &str, span: Span) -> Option<TestMarker> {
    let trimmed = content.trim();

    if trimmed == "test: code" || trimmed.starts_with("test: code ") {
        return Some(TestMarker {
            kind: TestMarkerKind::Code {
                backlinks: Vec::new(),
            },
            span,
        });
    }

    if let Some(reason) = trimmed.strip_prefix("manual:") {
        let reason = reason.trim().to_string();
        return Some(TestMarker {
            kind: TestMarkerKind::Manual { reason },
            span,
        });
    }

    if let Some(reason) = trimmed.strip_prefix("skip:") {
        let reason = reason.trim().to_string();
        return Some(TestMarker {
            kind: TestMarkerKind::Skip { reason },
            span,
        });
    }

    None
}

/// Consume `> - path:line` backlink items following a `> test: code` line.
fn consume_backlinks(elements: &[Element], cursor: &mut usize) -> Vec<Backlink> {
    let mut backlinks = Vec::new();
    while *cursor < elements.len() {
        match &elements[*cursor] {
            Element::BlockQuoteItem { content, span } if content.starts_with("- ") => {
                let path = content[2..].trim().to_string();
                backlinks.push(Backlink { path, span: *span });
                *cursor += 1;
            }
            _ => break,
        }
    }
    backlinks
}
