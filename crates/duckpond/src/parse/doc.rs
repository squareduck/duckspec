use crate::artifact::doc::*;
use crate::error::ParseError;
use crate::parse::{BlockKind, Element, Span};

/// Parse a sequence of L1 elements into a [`Document`].
///
/// Used for: capability docs, codex entries, `project.md`, proposals, designs.
pub fn parse_document(elements: &[Element]) -> Result<Document, Vec<ParseError>> {
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

    // -- Description (blocks before first heading) ---------------------------
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

    // -- Sections (heading tree) ---------------------------------------------
    let sections = parse_sections(elements, &mut cursor, 2);

    if errors.is_empty() {
        Ok(Document {
            title,
            title_span,
            summary,
            summary_span,
            description,
            sections,
        })
    } else {
        Err(errors)
    }
}

/// Recursively parse sections at or deeper than `min_level`.
///
/// A heading at `min_level` starts a new section at this level. Headings
/// deeper than `min_level` become children. Headings shallower than
/// `min_level` are not consumed (they belong to a parent).
fn parse_sections(elements: &[Element], cursor: &mut usize, min_level: u8) -> Vec<Section> {
    let mut sections = Vec::new();

    while *cursor < elements.len() {
        match &elements[*cursor] {
            Element::Heading {
                level,
                content,
                span,
            } if *level >= min_level => {
                let level = *level;
                let heading = content.clone();
                let heading_span = *span;
                *cursor += 1;

                // Collect body elements until the next heading.
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

                // Recurse into child sections (deeper level).
                let children = parse_sections(elements, cursor, level + 1);

                sections.push(Section {
                    heading,
                    level,
                    heading_span,
                    body,
                    children,
                });
            }
            Element::Heading { level, .. } if *level < min_level => {
                // This heading belongs to a parent level — stop.
                break;
            }
            _ => {
                // Shouldn't happen in well-formed input (body elements before
                // any heading at this level), but handle gracefully.
                *cursor += 1;
            }
        }
    }

    sections
}
