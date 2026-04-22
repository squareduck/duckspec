use crate::artifact::step::*;
use crate::error::ParseError;
use crate::parse::{BlockKind, Element, Span};

/// Parse a sequence of L1 elements into a [`Step`].
pub fn parse_step(elements: &[Element]) -> Result<Step, Vec<ParseError>> {
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

    let slug = slugify(&title);

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
            Element::Heading { .. } => break,
            elem => {
                description.push(elem.clone());
                cursor += 1;
            }
        }
    }

    // -- Named sections ------------------------------------------------------
    let mut prerequisites = None;
    let mut context = None;
    let mut tasks: Option<(Vec<Task>, Span)> = None;
    let mut outcomes = None;

    while cursor < elements.len() {
        match &elements[cursor] {
            Element::Heading {
                level: 2,
                content,
                span,
            } => {
                let section_name = content.trim();
                let section_span = *span;
                cursor += 1;

                match section_name {
                    "Prerequisites" => {
                        prerequisites = Some(parse_prerequisites(elements, &mut cursor));
                    }
                    "Context" => {
                        context = Some(collect_body(elements, &mut cursor));
                    }
                    "Tasks" => {
                        let (parsed_tasks, task_errors) =
                            parse_tasks(elements, &mut cursor, section_span);
                        errors.extend(task_errors);
                        tasks = Some((parsed_tasks, section_span));
                    }
                    "Outcomes" => {
                        outcomes = Some(collect_body(elements, &mut cursor));
                    }
                    _ => {
                        errors.push(ParseError::UnknownStepSection {
                            name: section_name.to_string(),
                            span: section_span,
                        });
                        // Skip body of unknown section.
                        collect_body(elements, &mut cursor);
                    }
                }
            }
            _ => {
                cursor += 1;
            }
        }
    }

    // -- Validate Tasks section exists and is non-empty ----------------------
    let tasks = match tasks {
        Some((task_list, section_span)) => {
            if task_list.is_empty() {
                errors.push(ParseError::EmptyTasksSection { span: section_span });
            }
            task_list
        }
        None => {
            errors.push(ParseError::MissingTasksSection { span: title_span });
            Vec::new()
        }
    };

    if errors.is_empty() {
        Ok(Step {
            title,
            title_span,
            slug,
            summary,
            summary_span,
            description,
            prerequisites,
            context,
            tasks,
            outcomes,
        })
    } else {
        Err(errors)
    }
}

// ---------------------------------------------------------------------------
// Section parsers
// ---------------------------------------------------------------------------

fn parse_prerequisites(elements: &[Element], cursor: &mut usize) -> Vec<Prerequisite> {
    let mut prereqs = Vec::new();

    while *cursor < elements.len() {
        match &elements[*cursor] {
            Element::Heading { .. } => break,
            Element::ListItem { content, span, .. } => {
                let (checked, text) = parse_checkbox(content);
                let kind = if let Some(slug) = text.strip_prefix("@step ") {
                    PrerequisiteKind::StepRef {
                        slug: slug.trim().to_string(),
                    }
                } else {
                    PrerequisiteKind::Freeform {
                        text: text.to_string(),
                    }
                };
                prereqs.push(Prerequisite {
                    kind,
                    checked,
                    span: *span,
                });
                *cursor += 1;
            }
            _ => {
                *cursor += 1;
            }
        }
    }

    prereqs
}

fn parse_tasks(
    elements: &[Element],
    cursor: &mut usize,
    _section_span: Span,
) -> (Vec<Task>, Vec<ParseError>) {
    let mut tasks = Vec::new();
    let mut errors = Vec::new();

    while *cursor < elements.len() {
        match &elements[*cursor] {
            Element::Heading { .. } => break,
            Element::ListItem {
                content,
                indent,
                span,
                ..
            } if *indent == 0 => {
                let (checked, text) = parse_checkbox(content);
                let text = strip_numeric_prefix(&text);
                let content_parsed = parse_task_content(&text);
                let mut task = Task {
                    content: content_parsed,
                    checked,
                    span: *span,
                    subtasks: Vec::new(),
                };
                *cursor += 1;

                // Collect subtasks (indent > 0).
                while *cursor < elements.len() {
                    match &elements[*cursor] {
                        Element::ListItem {
                            content,
                            indent,
                            span,
                            ..
                        } if *indent > 0 => {
                            if *indent > 4 {
                                errors.push(ParseError::SubtaskTooDeep { span: *span });
                            }
                            let (checked, text) = parse_checkbox(content);
                            let text = strip_numeric_prefix(&text);
                            let content_parsed = parse_task_content(&text);
                            task.subtasks.push(Subtask {
                                content: content_parsed,
                                checked,
                                span: *span,
                            });
                            *cursor += 1;
                        }
                        _ => break,
                    }
                }

                tasks.push(task);
            }
            _ => {
                *cursor += 1;
            }
        }
    }

    (tasks, errors)
}

/// Collect body elements until the next heading.
fn collect_body(elements: &[Element], cursor: &mut usize) -> Vec<Element> {
    let mut body = Vec::new();
    while *cursor < elements.len() {
        match &elements[*cursor] {
            Element::Heading { .. } => break,
            elem => {
                body.push(elem.clone());
                *cursor += 1;
            }
        }
    }
    body
}

// ---------------------------------------------------------------------------
// Content helpers
// ---------------------------------------------------------------------------

/// Parse `[ ] text` or `[x] text` at the start of a list item content.
/// Returns `(checked, remaining_text)`.
fn parse_checkbox(content: &str) -> (bool, String) {
    let trimmed = content.trim_start();
    if let Some(rest) = trimmed
        .strip_prefix("[x]")
        .or_else(|| trimmed.strip_prefix("[X]"))
    {
        (true, rest.trim_start().to_string())
    } else if let Some(rest) = trimmed.strip_prefix("[ ]") {
        (false, rest.trim_start().to_string())
    } else {
        (false, content.to_string())
    }
}

/// Strip a leading numeric prefix like `1. `, `2. `, `1.1 `, etc.
fn strip_numeric_prefix(text: &str) -> String {
    let trimmed = text.trim_start();
    // Match patterns like "1. ", "12. ", "1.1 ", "1.2 "
    let mut chars = trimmed.char_indices().peekable();

    // Must start with a digit.
    match chars.peek() {
        Some((_, c)) if c.is_ascii_digit() => {
            chars.next();
        }
        _ => return text.to_string(),
    }

    // Consume more digits and dots (for "1.1" style).
    while let Some((_, c)) = chars.peek() {
        if c.is_ascii_digit() || *c == '.' {
            chars.next();
        } else {
            break;
        }
    }

    // Must end with a space (already consumed the trailing digit/dot).
    match chars.peek() {
        Some((i, ' ')) => trimmed[i + 1..].trim_start().to_string(),
        Some((i, _)) => {
            // Check if the char before was a dot followed by space: "1. "
            // We already advanced past the dot, check current char.
            if trimmed.as_bytes().get(i.wrapping_sub(1)) == Some(&b'.') {
                // Actually we need a different approach: the dot was consumed,
                // now we need a space.
                text.to_string()
            } else {
                text.to_string()
            }
        }
        None => text.to_string(),
    }
}

/// Parse a task's text content into either a `@spec` reference or freeform text.
fn parse_task_content(text: &str) -> TaskContent {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("@spec ") {
        // Format: @spec <capability-path> <Requirement Name>: <Scenario Name>
        let rest = rest.trim();
        // Find the first space to separate capability path.
        if let Some(space_idx) = rest.find(' ') {
            let capability = rest[..space_idx].to_string();
            let rest = rest[space_idx + 1..].trim();
            // Find the first colon to separate requirement from scenario.
            if let Some(colon_idx) = rest.find(':') {
                let requirement = rest[..colon_idx].trim().to_string();
                let scenario = rest[colon_idx + 1..].trim().to_string();
                return TaskContent::SpecRef {
                    capability,
                    requirement,
                    scenario,
                };
            }
        }
    }
    TaskContent::Freeform {
        text: text.to_string(),
    }
}
