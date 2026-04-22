pub mod delta;
pub mod doc;
pub mod spec;
pub mod step;

use crate::format::{prose, table};
use crate::parse::{BlockKind, Element, ListMarker};

/// Render a single element back to markdown text, reflowing prose to `width`.
///
/// For `ListItem` elements this renders the bullet form. Numbered list items
/// require their position within the run to render correctly; use
/// [`render_body`] when sequential numbering matters.
pub(crate) fn render_element(elem: &Element, width: usize) -> String {
    match elem {
        Element::Heading { level, content, .. } => {
            let hashes = "#".repeat(*level as usize);
            format!("{hashes} {content}")
        }
        Element::Block {
            content,
            kind: BlockKind::Paragraph,
            ..
        } => render_paragraph(content, width),
        Element::Block {
            content,
            kind: BlockKind::CodeBlock,
            ..
        } => content.clone(),
        Element::ListItem {
            content,
            indent,
            marker: ListMarker::Bullet,
            ..
        } => render_bullet_item(content, *indent, width),
        Element::ListItem {
            content,
            indent,
            marker: ListMarker::Numbered,
            ..
        } => render_numbered_item(content, *indent, 1, width),
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
/// Consecutive list items with the same marker form a list run. Within a
/// numbered run, items are sequentially renumbered starting at `1`. A run
/// is rendered "loose" (blank lines between items) when at least one
/// rendered item spans multiple lines, otherwise "tight". Block-quote items
/// are always tight. Marker switches and all other adjacencies use a blank
/// line separator.
pub(crate) fn render_body(elements: &[Element], width: usize) -> String {
    let mut rendered: Vec<String> = vec![String::new(); elements.len()];
    let mut in_loose_list = vec![false; elements.len()];

    let mut i = 0;
    while i < elements.len() {
        if let Some(start_marker) = list_marker(&elements[i]) {
            // Find end of run (consecutive list items with same marker).
            let start = i;
            let mut end = i + 1;
            while end < elements.len() && list_marker(&elements[end]) == Some(start_marker) {
                end += 1;
            }

            // Render items, sequentially numbering for ordered lists.
            let mut number = 1usize;
            for k in start..end {
                if let Element::ListItem {
                    content,
                    indent,
                    marker,
                    ..
                } = &elements[k]
                {
                    rendered[k] = match marker {
                        ListMarker::Bullet => render_bullet_item(content, *indent, width),
                        ListMarker::Numbered => {
                            let s = render_numbered_item(content, *indent, number, width);
                            number += 1;
                            s
                        }
                    };
                }
            }

            // Loose detection: any item in the run is multi-line.
            if rendered[start..end].iter().any(|s| s.contains('\n')) {
                in_loose_list[start..end].fill(true);
            }

            i = end;
        } else {
            rendered[i] = render_element(&elements[i], width);
            i += 1;
        }
    }

    let mut out = String::new();
    for i in 0..elements.len() {
        if i > 0 {
            out.push_str(separator(&elements[i - 1], &elements[i], in_loose_list[i]));
        }
        out.push_str(&rendered[i]);
    }
    out
}

/// Reflow a free-form paragraph string at the given width. If the content is
/// a GFM table it is wrapped in a plain code fence so the reflower never
/// mangles it.
pub(crate) fn render_paragraph(content: &str, width: usize) -> String {
    if table::is_gfm_table(content) {
        format!("```\n{content}\n```")
    } else {
        prose::reflow(content, width)
    }
}

fn render_bullet_item(content: &str, indent: usize, width: usize) -> String {
    render_with_marker(content, indent, "- ", width)
}

fn render_numbered_item(content: &str, indent: usize, number: usize, width: usize) -> String {
    let marker = format!("{number}. ");
    render_with_marker(content, indent, &marker, width)
}

/// Reflow `content` and emit it with `marker_prefix` on the first line and
/// hang-indented continuations aligned under the column after the marker.
fn render_with_marker(content: &str, indent: usize, marker_prefix: &str, width: usize) -> String {
    let pad = " ".repeat(indent);
    let content_indent = indent + marker_prefix.chars().count();
    let avail = width.saturating_sub(content_indent).max(1);
    let reflowed = prose::reflow(content, avail);

    let cont_pad = " ".repeat(content_indent);
    let mut out = String::new();
    for (i, line) in reflowed.split('\n').enumerate() {
        if i == 0 {
            out.push_str(&pad);
            out.push_str(marker_prefix);
            out.push_str(line);
        } else {
            out.push('\n');
            out.push_str(&cont_pad);
            out.push_str(line);
        }
    }
    out
}

fn list_marker(elem: &Element) -> Option<ListMarker> {
    match elem {
        Element::ListItem { marker, .. } => Some(*marker),
        _ => None,
    }
}

fn separator(prev: &Element, next: &Element, next_in_loose_list: bool) -> &'static str {
    match (prev, next) {
        (
            Element::ListItem { marker: m1, .. },
            Element::ListItem { marker: m2, .. },
        ) => {
            if m1 != m2 || next_in_loose_list {
                "\n\n"
            } else {
                "\n"
            }
        }
        (Element::BlockQuoteItem { .. }, Element::BlockQuoteItem { .. }) => "\n",
        _ => "\n\n",
    }
}
