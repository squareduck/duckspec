pub mod delta;
pub mod doc;
pub mod spec;
pub mod step;

/// Byte offset range in the source string.
///
/// Compatible with `miette::SourceSpan` via `(offset, length)`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub offset: usize,
    pub length: usize,
}

impl std::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.offset, self.offset + self.length)
    }
}

impl From<Span> for miette::SourceSpan {
    fn from(s: Span) -> Self {
        (s.offset, s.length).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    CodeBlock,
}

/// Which marker a list item uses in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListMarker {
    /// `- text` — unordered bullet.
    Bullet,
    /// `1. text`, `2. text`, ... — ordered/numbered. The original number is
    /// dropped at parse time; the renderer re-numbers from 1 within each
    /// run.
    Numbered,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Element {
    Heading {
        level: u8,
        content: String,
        span: Span,
    },
    Block {
        content: String,
        kind: BlockKind,
        span: Span,
    },
    ListItem {
        content: String,
        indent: usize,
        marker: ListMarker,
        span: Span,
    },
    BlockQuoteItem {
        content: String,
        span: Span,
    },
}

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Element::Heading { level, content, .. } => {
                let hashes = "#".repeat(*level as usize);
                write!(f, "{hashes} {content}")
            }
            Element::Block {
                content,
                kind: BlockKind::Paragraph,
                ..
            } => write!(f, "{content}"),
            Element::Block {
                content,
                kind: BlockKind::CodeBlock,
                ..
            } => write!(f, "{content}"),
            Element::ListItem {
                content,
                indent,
                marker,
                ..
            } => {
                let pad = " ".repeat(*indent);
                let marker_text = match marker {
                    ListMarker::Bullet => "-",
                    ListMarker::Numbered => "1.",
                };
                write!(f, "{pad}{marker_text} {content}")
            }
            Element::BlockQuoteItem { content, .. } => {
                if content.is_empty() {
                    write!(f, ">")
                } else {
                    write!(f, "> {content}")
                }
            }
        }
    }
}

impl Element {
    pub fn span(&self) -> Span {
        match self {
            Element::Heading { span, .. }
            | Element::Block { span, .. }
            | Element::ListItem { span, .. }
            | Element::BlockQuoteItem { span, .. } => *span,
        }
    }
}

// ---------------------------------------------------------------------------
// Layer 1: Line-by-line state machine
// ---------------------------------------------------------------------------

/// Parse a markdown source string into a flat sequence of [`Element`]s.
///
/// This is an infallible operation — any input produces a valid sequence.
/// Structural validation is the job of the Layer 2 artifact parsers.
pub fn parse_elements(source: &str) -> Vec<Element> {
    let mut elements: Vec<Element> = Vec::new();
    let mut state = L1State::Normal;

    let mut line_start = 0;
    let lines: Vec<&str> = source.split('\n').collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let line_end = if line_idx + 1 < lines.len() {
            line_start + line.len() + 1 // +1 for the \n
        } else {
            line_start + line.len()
        };

        state = advance(source, &mut elements, state, line, line_start, line_end);
        line_start = line_end;
    }

    // Flush any pending state.
    flush(source, &mut elements, state);

    elements
}

#[derive(Debug)]
enum L1State {
    Normal,
    InParagraph {
        start: usize,
        end: usize,
    },
    InCodeBlock {
        start: usize,
        end: usize,
    },
    InListItem {
        start: usize,
        end: usize,
        indent: usize,
        content_start: usize,
        marker: ListMarker,
    },
}

fn advance(
    source: &str,
    elements: &mut Vec<Element>,
    state: L1State,
    line: &str,
    line_start: usize,
    line_end: usize,
) -> L1State {
    match state {
        L1State::InCodeBlock { start, .. } => {
            if line.trim_start() == "```" || line.trim_start().starts_with("``` ") {
                // End of code block — include the closing fence.
                let end = line_end;
                let span_text = &source[start..end];
                elements.push(Element::Block {
                    content: span_text.trim_end_matches('\n').to_string(),
                    kind: BlockKind::CodeBlock,
                    span: Span {
                        offset: start,
                        length: end - start,
                    },
                });
                L1State::Normal
            } else {
                L1State::InCodeBlock {
                    start,
                    end: line_end,
                }
            }
        }

        L1State::InListItem {
            start,
            end,
            indent,
            content_start,
            marker,
        } => {
            // A new list item (at any indent) terminates the current one.
            // Check this before continuation to avoid swallowing nested items.
            if is_list_item(line)
                || line.trim().is_empty()
                || is_heading(line)
                || is_block_quote(line)
                || is_code_fence(line)
            {
                flush_list_item(source, elements, start, end, indent, content_start, marker);
                advance(
                    source,
                    elements,
                    L1State::Normal,
                    line,
                    line_start,
                    line_end,
                )
            } else if is_continuation(line, content_start) {
                // A continuation line is indented beyond the bullet content start.
                L1State::InListItem {
                    start,
                    end: line_end,
                    indent,
                    content_start,
                    marker,
                }
            } else {
                // Flush the current list item and reprocess.
                flush_list_item(source, elements, start, end, indent, content_start, marker);
                advance(
                    source,
                    elements,
                    L1State::Normal,
                    line,
                    line_start,
                    line_end,
                )
            }
        }

        L1State::InParagraph { start, .. } => {
            if line.trim().is_empty() {
                // End paragraph.
                flush(
                    source,
                    elements,
                    L1State::InParagraph {
                        start,
                        end: line_start,
                    },
                );
                L1State::Normal
            } else if is_heading(line)
                || is_list_item(line)
                || is_block_quote(line)
                || is_code_fence(line)
            {
                // Flush paragraph, reprocess.
                flush(
                    source,
                    elements,
                    L1State::InParagraph {
                        start,
                        end: line_start,
                    },
                );
                advance(
                    source,
                    elements,
                    L1State::Normal,
                    line,
                    line_start,
                    line_end,
                )
            } else {
                L1State::InParagraph {
                    start,
                    end: line_end,
                }
            }
        }

        L1State::Normal => {
            if line.trim().is_empty() {
                L1State::Normal
            } else if is_code_fence(line) {
                L1State::InCodeBlock {
                    start: line_start,
                    end: line_end,
                }
            } else if let Some((level, content)) = parse_heading(line) {
                elements.push(Element::Heading {
                    level,
                    content: content.to_string(),
                    span: Span {
                        offset: line_start,
                        length: line_end - line_start,
                    },
                });
                L1State::Normal
            } else if is_block_quote(line) {
                let content = line.trim_start().strip_prefix('>').unwrap();
                let content = content.strip_prefix(' ').unwrap_or(content);
                elements.push(Element::BlockQuoteItem {
                    content: content.to_string(),
                    span: Span {
                        offset: line_start,
                        length: line_end - line_start,
                    },
                });
                L1State::Normal
            } else if let Some((indent, content_offset, marker)) = parse_list_item_start(line) {
                L1State::InListItem {
                    start: line_start,
                    end: line_end,
                    indent,
                    content_start: content_offset,
                    marker,
                }
            } else {
                // Start a paragraph.
                L1State::InParagraph {
                    start: line_start,
                    end: line_end,
                }
            }
        }
    }
}

fn flush(source: &str, elements: &mut Vec<Element>, state: L1State) {
    match state {
        L1State::Normal => {}
        L1State::InParagraph { start, end } => {
            let text = &source[start..end];
            let text = text.trim_end_matches('\n');
            if !text.is_empty() {
                elements.push(Element::Block {
                    content: text.to_string(),
                    kind: BlockKind::Paragraph,
                    span: Span {
                        offset: start,
                        length: end - start,
                    },
                });
            }
        }
        L1State::InCodeBlock { start, end } => {
            // Unclosed code block — emit what we have.
            let text = &source[start..end];
            elements.push(Element::Block {
                content: text.trim_end_matches('\n').to_string(),
                kind: BlockKind::CodeBlock,
                span: Span {
                    offset: start,
                    length: end - start,
                },
            });
        }
        L1State::InListItem {
            start,
            end,
            indent,
            content_start,
            marker,
        } => {
            flush_list_item(source, elements, start, end, indent, content_start, marker);
        }
    }
}

fn flush_list_item(
    source: &str,
    elements: &mut Vec<Element>,
    start: usize,
    end: usize,
    indent: usize,
    content_start: usize,
    marker: ListMarker,
) {
    let raw = &source[start..end];
    let raw = raw.trim_end_matches('\n');

    // The first line's content starts at `content_start` columns in.
    // Continuation lines are dedented to align with the first line's content.
    let first_line_end = raw.find('\n').unwrap_or(raw.len());
    let first_content = &raw[content_start..first_line_end];

    let mut content = first_content.to_string();
    if let Some(rest) = raw.get(first_line_end + 1..) {
        for cont_line in rest.split('\n') {
            content.push('\n');
            // Strip up to `content_start` columns of leading whitespace.
            if cont_line.len() >= content_start {
                let stripped = &cont_line[..content_start];
                if stripped.trim().is_empty() {
                    content.push_str(&cont_line[content_start..]);
                } else {
                    content.push_str(cont_line.trim_start());
                }
            } else {
                content.push_str(cont_line.trim_start());
            }
        }
    }

    elements.push(Element::ListItem {
        content,
        indent,
        marker,
        span: Span {
            offset: start,
            length: end - start,
        },
    });
}

// ---------------------------------------------------------------------------
// Line classification helpers
// ---------------------------------------------------------------------------

fn is_heading(line: &str) -> bool {
    parse_heading(line).is_some()
}

fn parse_heading(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim_start();
    let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if let Some(stripped) = rest.strip_prefix(' ') {
        Some((hashes as u8, stripped.trim_end()))
    } else {
        None
    }
}

fn is_code_fence(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

fn is_block_quote(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("> ") || trimmed == ">"
}

fn is_list_item(line: &str) -> bool {
    parse_list_item_start(line).is_some()
}

/// Returns `(indent_level, content_start_column, marker)` for a line starting
/// a list item. Recognizes both bullet (`- text`) and numbered (`N. text`)
/// markers.
///
/// `indent_level` is the number of leading spaces before the marker.
/// `content_start_column` is the byte offset within the line where content
/// begins (after the marker), used for aligning continuation lines.
fn parse_list_item_start(line: &str) -> Option<(usize, usize, ListMarker)> {
    let indent = line.bytes().take_while(|&b| b == b' ').count();
    let rest = &line[indent..];

    if rest.starts_with("- ") {
        return Some((indent, indent + 2, ListMarker::Bullet));
    }

    // Numbered marker: one or more digits, then ".", then a space.
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && i + 1 < bytes.len() && bytes[i] == b'.' && bytes[i + 1] == b' ' {
        // i = number of digits; marker length = digits + ". " = i + 2
        return Some((indent, indent + i + 2, ListMarker::Numbered));
    }

    None
}

/// Check if a line is a continuation of a list item whose content starts at
/// `content_start` columns.
fn is_continuation(line: &str, content_start: usize) -> bool {
    let leading = line.bytes().take_while(|&b| b == b' ').count();
    leading >= content_start && !line.trim().is_empty()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn elements(source: &str) -> Vec<Element> {
        parse_elements(source)
    }

    // @spec parse/elements Element model and source spans: Empty input produces no elements
    #[test]
    fn empty_input() {
        assert!(elements("").is_empty());
    }

    #[test]
    fn single_heading() {
        let elems = elements("# Hello");
        assert_eq!(elems.len(), 1);
        assert!(
            matches!(&elems[0], Element::Heading { level: 1, content, .. } if content == "Hello")
        );
    }

    // @spec parse/elements ATX heading classification: ATX headings at levels 1 through 6
    #[test]
    fn heading_levels() {
        let src = "# H1\n## H2\n### H3\n#### H4";
        let elems = elements(src);
        assert_eq!(elems.len(), 4);
        assert!(matches!(&elems[0], Element::Heading { level: 1, content, .. } if content == "H1"));
        assert!(matches!(&elems[1], Element::Heading { level: 2, content, .. } if content == "H2"));
        assert!(matches!(&elems[2], Element::Heading { level: 3, content, .. } if content == "H3"));
        assert!(matches!(&elems[3], Element::Heading { level: 4, content, .. } if content == "H4"));
    }

    #[test]
    fn paragraph_single_line() {
        let elems = elements("Hello world");
        assert_eq!(elems.len(), 1);
        assert!(
            matches!(&elems[0], Element::Block { content, kind: BlockKind::Paragraph, .. } if content == "Hello world")
        );
    }

    // @spec parse/elements Paragraph aggregation: Multi-line paragraph stays a single block
    #[test]
    fn paragraph_multi_line() {
        let src = "Line one\nLine two\nLine three";
        let elems = elements(src);
        assert_eq!(elems.len(), 1);
        assert!(
            matches!(&elems[0], Element::Block { content, kind: BlockKind::Paragraph, .. } if content == "Line one\nLine two\nLine three")
        );
    }

    // @spec parse/elements Paragraph aggregation: Blank line separates paragraphs
    #[test]
    fn paragraphs_separated_by_blank_line() {
        let src = "First paragraph\n\nSecond paragraph";
        let elems = elements(src);
        assert_eq!(elems.len(), 2);
        assert!(
            matches!(&elems[0], Element::Block { content, kind: BlockKind::Paragraph, .. } if content == "First paragraph")
        );
        assert!(
            matches!(&elems[1], Element::Block { content, kind: BlockKind::Paragraph, .. } if content == "Second paragraph")
        );
    }

    // @spec parse/elements Fenced code blocks: Code block preserves content verbatim
    #[test]
    fn code_block() {
        let src = "```rust\nfn main() {}\n```";
        let elems = elements(src);
        assert_eq!(elems.len(), 1);
        assert!(
            matches!(&elems[0], Element::Block { content, kind: BlockKind::CodeBlock, .. } if content == "```rust\nfn main() {}\n```")
        );
    }

    #[test]
    fn code_block_preserves_blank_lines() {
        let src = "```\nline 1\n\nline 2\n```";
        let elems = elements(src);
        assert_eq!(elems.len(), 1);
        assert!(
            matches!(&elems[0], Element::Block { content, kind: BlockKind::CodeBlock, .. } if content == "```\nline 1\n\nline 2\n```")
        );
    }

    #[test]
    fn simple_list_items() {
        let src = "- First\n- Second\n- Third";
        let elems = elements(src);
        assert_eq!(elems.len(), 3);
        assert!(
            matches!(&elems[0], Element::ListItem { content, indent: 0, .. } if content == "First")
        );
        assert!(
            matches!(&elems[1], Element::ListItem { content, indent: 0, .. } if content == "Second")
        );
        assert!(
            matches!(&elems[2], Element::ListItem { content, indent: 0, .. } if content == "Third")
        );
    }

    // @spec parse/elements List item recognition: Indent records nesting level
    #[test]
    fn nested_list_items() {
        let src = "- Outer\n  - Inner";
        let elems = elements(src);
        assert_eq!(elems.len(), 2);
        assert!(
            matches!(&elems[0], Element::ListItem { content, indent: 0, .. } if content == "Outer")
        );
        assert!(
            matches!(&elems[1], Element::ListItem { content, indent: 2, .. } if content == "Inner")
        );
    }

    // @spec parse/elements List item recognition: Continuation lines aligned with content are absorbed
    #[test]
    fn list_item_with_continuation() {
        let src = "- First line\n  continuation line";
        let elems = elements(src);
        assert_eq!(elems.len(), 1);
        assert!(
            matches!(&elems[0], Element::ListItem { content, indent: 0, .. } if content == "First line\ncontinuation line")
        );
    }

    // @spec parse/elements Block quote recognition: Each block quote line produces its own element
    #[test]
    fn block_quote_items() {
        let src = "> First line\n> Second line";
        let elems = elements(src);
        assert_eq!(elems.len(), 2);
        assert!(
            matches!(&elems[0], Element::BlockQuoteItem { content, .. } if content == "First line")
        );
        assert!(
            matches!(&elems[1], Element::BlockQuoteItem { content, .. } if content == "Second line")
        );
    }

    // @spec parse/elements Block quote recognition: Block-quote-formatted list-like content stays a block quote
    #[test]
    fn block_quote_with_list() {
        let src = "> test: code\n> - crates/foo.rs:42";
        let elems = elements(src);
        assert_eq!(elems.len(), 2);
        assert!(
            matches!(&elems[0], Element::BlockQuoteItem { content, .. } if content == "test: code")
        );
        assert!(
            matches!(&elems[1], Element::BlockQuoteItem { content, .. } if content == "- crates/foo.rs:42")
        );
    }

    // @spec parse/elements Element model and source spans: Mixed content produces an ordered sequence of distinct kinds
    #[test]
    fn mixed_content() {
        let src = "\
# Title

Summary paragraph

## Section

- Item one
- Item two

> A quote

Some prose";
        let elems = elements(src);
        assert_eq!(elems.len(), 7);
        assert!(
            matches!(&elems[0], Element::Heading { level: 1, content, .. } if content == "Title")
        );
        assert!(matches!(
            &elems[1],
            Element::Block {
                kind: BlockKind::Paragraph,
                ..
            }
        ));
        assert!(matches!(&elems[2], Element::Heading { level: 2, .. }));
        assert!(matches!(&elems[3], Element::ListItem { content, .. } if content == "Item one"));
        assert!(matches!(&elems[4], Element::ListItem { content, .. } if content == "Item two"));
        assert!(matches!(&elems[5], Element::BlockQuoteItem { .. }));
        assert!(matches!(
            &elems[6],
            Element::Block {
                kind: BlockKind::Paragraph,
                ..
            }
        ));
    }

    // @spec parse/elements ATX heading classification: A heading terminates the preceding paragraph
    #[test]
    fn heading_terminates_paragraph() {
        let src = "Some text\n## Heading";
        let elems = elements(src);
        assert_eq!(elems.len(), 2);
        assert!(matches!(
            &elems[0],
            Element::Block {
                kind: BlockKind::Paragraph,
                ..
            }
        ));
        assert!(matches!(&elems[1], Element::Heading { level: 2, .. }));
    }

    // @spec parse/elements Element model and source spans: Element spans match byte offsets in the source
    #[test]
    fn span_offsets() {
        let src = "# Title\n\nBody";
        let elems = elements(src);
        assert_eq!(
            elems[0].span(),
            Span {
                offset: 0,
                length: 8
            }
        );
        // "Body" starts at byte 9 (after "# Title\n\n")
        assert_eq!(elems[1].span().offset, 9);
    }

    // @spec parse/elements ATX heading classification: Hashes without a following space become paragraphs
    #[test]
    fn not_a_heading() {
        // No space after # is not a heading.
        let elems = elements("#hashtag");
        assert_eq!(elems.len(), 1);
        assert!(matches!(
            &elems[0],
            Element::Block {
                kind: BlockKind::Paragraph,
                ..
            }
        ));
    }

    #[test]
    fn list_followed_by_paragraph() {
        let src = "- Item\n\nParagraph";
        let elems = elements(src);
        assert_eq!(elems.len(), 2);
        assert!(matches!(&elems[0], Element::ListItem { .. }));
        assert!(matches!(
            &elems[1],
            Element::Block {
                kind: BlockKind::Paragraph,
                ..
            }
        ));
    }

    // @spec parse/elements List item recognition: Loose lists produce one ListItem per item
    #[test]
    fn loose_list_with_blank_line_separators() {
        // Items separated by blank lines parse as the same number of
        // ListItem elements as the tight equivalent — blank lines
        // simply flush each item.
        let src = "- one\n\n- two\n\n- three";
        let elems = elements(src);
        let list_items: Vec<_> = elems
            .iter()
            .filter(|e| matches!(e, Element::ListItem { .. }))
            .collect();
        assert_eq!(list_items.len(), 3);
    }

    #[test]
    fn loose_gwt_list_parses_as_three_clauses() {
        let src = "- **GIVEN** a user\n\n- **WHEN** they log in\n\n- **THEN** they get a token";
        let elems = elements(src);
        let list_items: Vec<_> = elems
            .iter()
            .filter(|e| matches!(e, Element::ListItem { .. }))
            .collect();
        assert_eq!(list_items.len(), 3);
    }

    #[test]
    fn numbered_list_items() {
        let src = "1. First\n2. Second\n3. Third";
        let elems = elements(src);
        assert_eq!(elems.len(), 3);
        for e in &elems {
            assert!(matches!(
                e,
                Element::ListItem {
                    marker: ListMarker::Numbered,
                    ..
                }
            ));
        }
    }

    #[test]
    fn numbered_list_with_continuation() {
        let src = "1. First line\n   continuation\n2. Second";
        let elems = elements(src);
        assert_eq!(elems.len(), 2);
        assert!(matches!(
            &elems[0],
            Element::ListItem {
                marker: ListMarker::Numbered,
                content,
                ..
            } if content == "First line\ncontinuation"
        ));
    }

    // @spec parse/elements List item recognition: Double-digit numbered markers are recognized
    #[test]
    fn double_digit_numbered_marker_works() {
        let src = "10. Tenth\n11. Eleventh";
        let elems = elements(src);
        assert_eq!(elems.len(), 2);
        assert!(matches!(
            &elems[0],
            Element::ListItem {
                marker: ListMarker::Numbered,
                ..
            }
        ));
    }

    // @spec parse/elements List item recognition: List look-alikes fall back to paragraphs
    #[test]
    fn bare_digit_dot_no_space_is_not_a_list() {
        // "1.no space" is just a paragraph.
        let src = "1.no space";
        let elems = elements(src);
        assert_eq!(elems.len(), 1);
        assert!(matches!(
            &elems[0],
            Element::Block {
                kind: BlockKind::Paragraph,
                ..
            }
        ));
    }

    #[test]
    fn version_number_in_paragraph_is_not_a_list() {
        // "1.0 version" — the "." has no following space-after-digits pattern.
        let src = "1.0 version";
        let elems = elements(src);
        assert_eq!(elems.len(), 1);
        assert!(matches!(
            &elems[0],
            Element::Block {
                kind: BlockKind::Paragraph,
                ..
            }
        ));
    }

    // @spec parse/elements List item recognition: Bullet and numbered markers are distinguished
    #[test]
    fn bullet_marker_recorded() {
        let src = "- one";
        let elems = elements(src);
        assert!(matches!(
            &elems[0],
            Element::ListItem {
                marker: ListMarker::Bullet,
                ..
            }
        ));
    }

    // @spec parse/elements Element model and source spans: Unclosed code block is flushed at end of input
    #[test]
    fn unclosed_code_block_flushes_at_eof() {
        let elems = elements("```rust\nfoo");
        assert_eq!(elems.len(), 1);
        assert!(matches!(
            &elems[0],
            Element::Block { kind: BlockKind::CodeBlock, content, .. }
            if content.starts_with("```rust") && content.ends_with("foo")
        ));
    }

    // @spec parse/elements Fenced code blocks: Code block preserves blank lines and info strings
    #[test]
    fn code_fence_with_info_string() {
        let elems = elements("```rust\nfn main() {}\n```");
        assert!(matches!(
            &elems[0],
            Element::Block { kind: BlockKind::CodeBlock, content, .. }
            if content.starts_with("```rust") && content.ends_with("```")
        ));
    }

    // @spec parse/elements Block quote recognition: A block quote terminates a list item
    #[test]
    fn block_quote_terminates_list_item() {
        let elems = elements("- item\n> quote");
        assert_eq!(elems.len(), 2);
        assert!(matches!(&elems[0], Element::ListItem { .. }));
        assert!(matches!(&elems[1], Element::BlockQuoteItem { .. }));
    }
}
