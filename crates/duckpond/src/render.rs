pub mod delta;
pub mod doc;
pub mod spec;
pub mod step;

use crate::parse::{BlockKind, Element};

/// Render a single element back to markdown text.
pub(crate) fn render_element(elem: &Element) -> String {
    match elem {
        Element::Heading { level, content, .. } => {
            let hashes = "#".repeat(*level as usize);
            format!("{hashes} {content}")
        }
        Element::Block {
            content,
            kind: BlockKind::Paragraph,
            ..
        } => content.clone(),
        Element::Block {
            content,
            kind: BlockKind::CodeBlock,
            ..
        } => content.clone(),
        Element::ListItem {
            content, indent, ..
        } => {
            let pad = " ".repeat(*indent);
            // Rejoin continuation lines with proper indentation.
            let content_indent = *indent + 2;
            let mut lines = content.lines();
            let mut out = format!("{pad}- {}", lines.next().unwrap_or(""));
            for line in lines {
                out.push('\n');
                if line.is_empty() {
                    // Preserve blank lines within continuation.
                } else {
                    out.push_str(&" ".repeat(content_indent));
                    out.push_str(line);
                }
            }
            out
        }
        Element::BlockQuoteItem { content, .. } => {
            if content.is_empty() {
                ">".to_string()
            } else {
                format!("> {content}")
            }
        }
    }
}

/// Render a sequence of elements with appropriate separators.
///
/// Consecutive list items and consecutive block quote items are joined with
/// a single newline (tight). All other adjacent pairs are separated by a
/// blank line.
pub(crate) fn render_body(elements: &[Element]) -> String {
    let mut out = String::new();
    for (i, elem) in elements.iter().enumerate() {
        if i > 0 {
            let sep = if should_be_tight(&elements[i - 1], elem) {
                "\n"
            } else {
                "\n\n"
            };
            out.push_str(sep);
        }
        out.push_str(&render_element(elem));
    }
    out
}

/// Two adjacent elements should be tight (no blank line) when they are both
/// list items or both block quote items.
fn should_be_tight(prev: &Element, next: &Element) -> bool {
    matches!(
        (prev, next),
        (Element::ListItem { .. }, Element::ListItem { .. })
            | (
                Element::BlockQuoteItem { .. },
                Element::BlockQuoteItem { .. }
            )
    )
}
