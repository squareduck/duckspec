use crate::artifact::delta::*;
use crate::artifact::doc::Section;
use crate::error::ParseError;
use crate::parse::{BlockKind, Element, Span};

/// Parse a sequence of L1 elements into a [`Delta`].
///
/// The parser accepts entries in any order and sorts them into canonical
/// order (`=` → `-` → `~` → `@` → `+`) as a post-processing step. The
/// returned `Delta` struct is always in canonical order.
pub fn parse_delta(elements: &[Element]) -> Result<Delta, Vec<ParseError>> {
    let mut errors = Vec::new();

    // -- H1 with marker -----------------------------------------------------
    let (marker, title, title_span, mut cursor) = match elements.first() {
        Some(Element::Heading {
            level: 1,
            content,
            span,
        }) => match parse_marked_heading(content, *span, &mut errors) {
            Some((m, t)) => {
                if m == DeltaMarker::Add {
                    errors.push(ParseError::AddOnH1 { span: *span });
                }
                (m, t, *span, 1)
            }
            None => (DeltaMarker::Anchor, content.clone(), *span, 1),
        },
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

    // -- Optional summary paragraph -----------------------------------------
    let (summary, summary_span) = match elements.get(cursor) {
        Some(Element::Block {
            kind: BlockKind::Paragraph,
            content,
            span,
        }) if !is_heading_next_or_end(elements, cursor) => {
            cursor += 1;
            (Some(content.clone()), Some(*span))
        }
        _ => (None, None),
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

    // -- H2 entries ----------------------------------------------------------
    let mut entries = Vec::new();

    while cursor < elements.len() {
        match &elements[cursor] {
            Element::Heading {
                level: 2,
                content,
                span,
            } => {
                let (entry, new_cursor) =
                    parse_h2_entry(elements, cursor, content, *span, &mut errors);
                entries.push(entry);
                cursor = new_cursor;
            }
            _ => {
                cursor += 1;
            }
        }
    }

    // -- Canonical sort ------------------------------------------------------
    entries.sort_by_key(|e| e.marker.order());
    for entry in &mut entries {
        if let DeltaChildren::Operations(ref mut ops) = entry.children {
            ops.sort_by_key(|c| c.marker.order());
        }
    }

    if errors.is_empty() {
        Ok(Delta {
            marker,
            title,
            title_span,
            summary,
            summary_span,
            description,
            entries,
        })
    } else {
        Err(errors)
    }
}

fn parse_h2_entry(
    elements: &[Element],
    start: usize,
    heading: &str,
    heading_span: Span,
    errors: &mut Vec<ParseError>,
) -> (DeltaEntry, usize) {
    let (entry_marker, entry_heading) = match parse_marked_heading(heading, heading_span, errors) {
        Some((m, t)) => (m, t),
        None => (DeltaMarker::Anchor, heading.to_string()),
    };

    let mut cursor = start + 1;
    let mut body = Vec::new();
    let mut rename_to = None;

    // For rename entries, the first paragraph line is the new name.
    if entry_marker == DeltaMarker::Rename {
        match elements.get(cursor) {
            Some(Element::Block {
                kind: BlockKind::Paragraph,
                content,
                span,
            }) => {
                let new_name = content.lines().next().unwrap_or("").trim().to_string();
                if new_name.is_empty() {
                    errors.push(ParseError::InvalidRenameEntry { span: *span });
                }
                rename_to = Some(new_name);
                cursor += 1;
            }
            _ => {
                errors.push(ParseError::InvalidRenameEntry { span: heading_span });
            }
        }
    }

    // Collect body elements until the next H2 or H3.
    while cursor < elements.len() {
        match &elements[cursor] {
            Element::Heading { level, .. } if *level <= 3 => break,
            Element::Heading { level, span, .. } if *level >= 4 => {
                errors.push(ParseError::HeadingTooDeep { span: *span });
                cursor += 1;
            }
            elem => {
                if entry_marker == DeltaMarker::Remove {
                    errors.push(ParseError::NonEmptyRemoveBody { span: elem.span() });
                }
                body.push(elem.clone());
                cursor += 1;
            }
        }
    }

    // Parse H3 children — the path depends on the parent marker.
    let children = match entry_marker {
        DeltaMarker::Anchor => {
            // Under @: children are delta operations (markers required).
            let ops = parse_operation_children(elements, &mut cursor, errors);
            DeltaChildren::Operations(ops)
        }
        DeltaMarker::Replace | DeltaMarker::Add => {
            // Under ~ and +: children are plain content (markers forbidden).
            let sections = parse_content_children(elements, &mut cursor, 3, errors);
            DeltaChildren::Content(sections)
        }
        DeltaMarker::Rename | DeltaMarker::Remove => {
            // No children expected for = and -.
            DeltaChildren::Content(vec![])
        }
    };

    (
        DeltaEntry {
            marker: entry_marker,
            heading: entry_heading,
            heading_span,
            body,
            rename_to,
            children,
        },
        cursor,
    )
}

// ---------------------------------------------------------------------------
// H3 children: two parsing paths
// ---------------------------------------------------------------------------

/// Parse H3 children as delta operations (under `@` entries).
/// Every heading must carry a marker.
fn parse_operation_children(
    elements: &[Element],
    cursor: &mut usize,
    errors: &mut Vec<ParseError>,
) -> Vec<DeltaChildEntry> {
    let mut children = Vec::new();

    while *cursor < elements.len() {
        match &elements[*cursor] {
            Element::Heading { level, .. } if *level <= 2 => break,
            Element::Heading {
                level: 3,
                content,
                span,
            } => {
                let (child, new_cursor) =
                    parse_h3_operation(elements, *cursor, content, *span, errors);
                children.push(child);
                *cursor = new_cursor;
            }
            Element::Heading { level, span, .. } if *level >= 4 => {
                errors.push(ParseError::HeadingTooDeep { span: *span });
                *cursor += 1;
            }
            _ => {
                *cursor += 1;
            }
        }
    }

    children
}

fn parse_h3_operation(
    elements: &[Element],
    start: usize,
    heading: &str,
    heading_span: Span,
    errors: &mut Vec<ParseError>,
) -> (DeltaChildEntry, usize) {
    let (child_marker, child_heading) = match parse_marked_heading(heading, heading_span, errors) {
        Some((m, t)) => {
            if m == DeltaMarker::Anchor {
                errors.push(ParseError::AnchorOnH3 { span: heading_span });
            }
            (m, t)
        }
        None => (DeltaMarker::Anchor, heading.to_string()),
    };

    let mut cursor = start + 1;
    let mut body = Vec::new();
    let mut rename_to = None;

    // For rename entries, first paragraph is the new name.
    if child_marker == DeltaMarker::Rename {
        match elements.get(cursor) {
            Some(Element::Block {
                kind: BlockKind::Paragraph,
                content,
                span,
            }) => {
                let new_name = content.lines().next().unwrap_or("").trim().to_string();
                if new_name.is_empty() {
                    errors.push(ParseError::InvalidRenameEntry { span: *span });
                }
                rename_to = Some(new_name);
                cursor += 1;
            }
            _ => {
                errors.push(ParseError::InvalidRenameEntry { span: heading_span });
            }
        }
    }

    while cursor < elements.len() {
        match &elements[cursor] {
            Element::Heading { level, .. } if *level <= 3 => break,
            Element::Heading { level, span, .. } if *level >= 4 => {
                errors.push(ParseError::HeadingTooDeep { span: *span });
                cursor += 1;
            }
            elem => {
                if child_marker == DeltaMarker::Remove {
                    errors.push(ParseError::NonEmptyRemoveBody { span: elem.span() });
                }
                body.push(elem.clone());
                cursor += 1;
            }
        }
    }

    (
        DeltaChildEntry {
            marker: child_marker,
            heading: child_heading,
            heading_span,
            body,
            rename_to,
        },
        cursor,
    )
}

/// Parse H3 children as plain content sections (under `~` and `+` entries).
/// Markers are forbidden — if a heading starts with a marker character, it's
/// a parse error.
fn parse_content_children(
    elements: &[Element],
    cursor: &mut usize,
    level: u8,
    errors: &mut Vec<ParseError>,
) -> Vec<Section> {
    let mut sections = Vec::new();

    while *cursor < elements.len() {
        match &elements[*cursor] {
            Element::Heading {
                level: hl,
                content,
                span,
            } if *hl == level => {
                // Check that the heading does NOT start with a marker.
                let trimmed = content.trim();
                if let Some(first) = trimmed.chars().next()
                    && DeltaMarker::from_char(first).is_some()
                {
                    errors.push(ParseError::MarkerOnContentChild { span: *span });
                }

                let heading = content.clone();
                let heading_span = *span;
                *cursor += 1;

                // Collect body until next heading at this level or shallower.
                let mut body = Vec::new();
                while *cursor < elements.len() {
                    match &elements[*cursor] {
                        Element::Heading { level: hl, .. } if *hl <= level => break,
                        Element::Heading {
                            level: hl, span, ..
                        } if *hl >= 4 => {
                            errors.push(ParseError::HeadingTooDeep { span: *span });
                            *cursor += 1;
                        }
                        elem => {
                            body.push(elem.clone());
                            *cursor += 1;
                        }
                    }
                }

                sections.push(Section {
                    heading,
                    level,
                    heading_span,
                    body,
                    children: vec![],
                });
            }
            Element::Heading { level: hl, .. } if *hl < level => break,
            _ => {
                *cursor += 1;
            }
        }
    }

    sections
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a heading line that carries a delta marker: `<marker> <text>`.
///
/// Returns `None` and pushes an error if the heading has no marker.
fn parse_marked_heading(
    heading: &str,
    span: Span,
    errors: &mut Vec<ParseError>,
) -> Option<(DeltaMarker, String)> {
    let trimmed = heading.trim();
    let mut chars = trimmed.chars();
    let first = chars.next()?;

    if let Some(marker) = DeltaMarker::from_char(first) {
        let rest = chars.as_str().trim();
        Some((marker, rest.to_string()))
    } else {
        errors.push(ParseError::MissingDeltaMarker { span });
        None
    }
}

fn is_heading_next_or_end(elements: &[Element], after_cursor: usize) -> bool {
    let next = after_cursor + 1;
    if next >= elements.len() {
        return true;
    }
    matches!(elements[next], Element::Heading { .. })
}
