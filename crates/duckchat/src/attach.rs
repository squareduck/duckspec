//! Shared walk of `[label](attach:<id>)` markers in a turn prompt.
//!
//! Providers encode the resulting [`Segment`]s into their own wire formats
//! (Anthropic content blocks for Claude, ACP content blocks for grok).

use std::collections::HashMap;

use crate::request::Attachment;

/// One piece of a user prompt after attach-marker resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Text(String),
    Image { media_type: String, bytes: Vec<u8> },
}

/// Walk `prompt` for markdown links of the form `[label](attach:<id>)`.
///
/// - Resolved `image/*` attachments become [`Segment::Image`].
/// - Resolved non-image attachments become a text fallback
///   (`[attachment: {label} ({n} bytes)]`).
/// - Unresolved, malformed, or non-attach links stay as literal text.
/// - Adjacent text spans are merged. An empty prompt yields a single empty
///   text segment so callers always have something to encode.
pub fn walk(prompt: &str, attachments: &HashMap<String, Attachment>) -> Vec<Segment> {
    let mut segments: Vec<Segment> = Vec::new();
    let mut cursor = 0usize;

    while let Some(open_rel) = prompt[cursor..].find('[') {
        let open = cursor + open_rel;
        let pre = &prompt[cursor..open];

        // Look for the matching `]` on the same line; bail to text on miss.
        let Some((_label, link_start)) = find_label_end(prompt, open) else {
            append_text(&mut segments, &prompt[cursor..=open]);
            cursor = open + 1;
            continue;
        };

        // Expect "(attach:" right after the label.
        let Some(id_start) = prompt[link_start..]
            .strip_prefix("(attach:")
            .map(|_| link_start + "(attach:".len())
        else {
            append_text(&mut segments, &prompt[cursor..=open]);
            cursor = open + 1;
            continue;
        };

        // Id terminates at `)` on the same line.
        let Some(id_len) = prompt[id_start..].find([')', '\n']) else {
            append_text(&mut segments, &prompt[cursor..=open]);
            cursor = open + 1;
            continue;
        };
        if prompt.as_bytes()[id_start + id_len] != b')' {
            append_text(&mut segments, &prompt[cursor..=open]);
            cursor = open + 1;
            continue;
        }
        let id = &prompt[id_start..id_start + id_len];
        let span_end = id_start + id_len + 1;

        match attachments.get(id) {
            Some(att) if att.media_type.starts_with("image/") => {
                append_text(&mut segments, pre);
                segments.push(Segment::Image {
                    media_type: att.media_type.clone(),
                    bytes: att.bytes.clone(),
                });
                cursor = span_end;
            }
            Some(att) => {
                // Non-image: keep label visible, drop the bytes (model has no
                // way to consume them in a content block).
                append_text(&mut segments, pre);
                append_text(
                    &mut segments,
                    &format!("[attachment: {} ({} bytes)]", att.label, att.bytes.len()),
                );
                cursor = span_end;
            }
            None => {
                // Unresolved id — keep the link literal in the text.
                append_text(&mut segments, &prompt[cursor..span_end]);
                cursor = span_end;
            }
        }
    }

    append_text(&mut segments, &prompt[cursor..]);
    if segments.is_empty() {
        segments.push(Segment::Text(String::new()));
    }
    segments
}

/// Append `s` as a text segment, merging with the previous segment when it is
/// also text. Empty strings are dropped.
fn append_text(segments: &mut Vec<Segment>, s: &str) {
    if s.is_empty() {
        return;
    }
    if let Some(Segment::Text(prev)) = segments.last_mut() {
        prev.push_str(s);
        return;
    }
    segments.push(Segment::Text(s.to_string()));
}

/// Returns `(label, position_after_closing_bracket)` if `prompt[open..]`
/// starts a markdown link label terminated by `]` before the next newline.
fn find_label_end(prompt: &str, open: usize) -> Option<(&str, usize)> {
    let after_open = open + 1;
    let rest = &prompt[after_open..];
    let close_rel = rest.find([']', '\n'])?;
    if rest.as_bytes()[close_rel] != b']' {
        return None;
    }
    let label = &rest[..close_rel];
    Some((label, after_open + close_rel + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(name: &str) -> Attachment {
        Attachment {
            label: name.to_string(),
            media_type: "image/png".to_string(),
            bytes: vec![1, 2, 3, 4],
        }
    }

    fn text_seg(segments: &[Segment], i: usize) -> &str {
        match &segments[i] {
            Segment::Text(t) => t,
            Segment::Image { .. } => panic!("expected text at {i}"),
        }
    }

    #[test]
    fn plain_text_yields_single_text_segment() {
        let segments = walk("hello world", &HashMap::new());
        assert_eq!(segments.len(), 1);
        assert_eq!(text_seg(&segments, 0), "hello world");
    }

    #[test]
    fn single_image_link_emits_image_segment() {
        let mut atts = HashMap::new();
        atts.insert("a1".to_string(), img("clip.png"));
        let segments = walk("look at [clip.png](attach:a1)!", &atts);
        assert_eq!(segments.len(), 3);
        assert_eq!(text_seg(&segments, 0), "look at ");
        match &segments[1] {
            Segment::Image { media_type, bytes } => {
                assert_eq!(media_type, "image/png");
                assert_eq!(bytes, &vec![1, 2, 3, 4]);
            }
            Segment::Text(_) => panic!("expected image"),
        }
        assert_eq!(text_seg(&segments, 2), "!");
    }

    #[test]
    fn two_links_interleaved_with_text() {
        let mut atts = HashMap::new();
        atts.insert("a".to_string(), img("a.png"));
        atts.insert("b".to_string(), img("b.png"));
        let segments = walk("first [a.png](attach:a) then [b.png](attach:b) done", &atts);
        assert_eq!(segments.len(), 5);
        assert_eq!(text_seg(&segments, 0), "first ");
        assert!(matches!(segments[1], Segment::Image { .. }));
        assert_eq!(text_seg(&segments, 2), " then ");
        assert!(matches!(segments[3], Segment::Image { .. }));
        assert_eq!(text_seg(&segments, 4), " done");
    }

    #[test]
    fn unresolved_id_falls_through_to_text() {
        let segments = walk("see [thing](attach:missing)", &HashMap::new());
        assert_eq!(segments.len(), 1);
        assert_eq!(text_seg(&segments, 0), "see [thing](attach:missing)");
    }

    #[test]
    fn unrelated_markdown_link_is_left_alone() {
        let segments = walk("see [docs](https://example.com)", &HashMap::new());
        assert_eq!(segments.len(), 1);
        assert_eq!(text_seg(&segments, 0), "see [docs](https://example.com)");
    }

    #[test]
    fn malformed_link_falls_through() {
        // No closing paren on the same line → not a link.
        let mut atts = HashMap::new();
        atts.insert("a".to_string(), img("a.png"));
        let segments = walk("oops [a.png](attach:a\nrest", &atts);
        assert_eq!(segments.len(), 1);
        assert_eq!(text_seg(&segments, 0), "oops [a.png](attach:a\nrest");
    }

    #[test]
    fn empty_prompt_yields_empty_text_segment() {
        let segments = walk("", &HashMap::new());
        assert_eq!(segments.len(), 1);
        assert_eq!(text_seg(&segments, 0), "");
    }

    #[test]
    fn non_image_attachment_becomes_text_fallback() {
        let mut atts = HashMap::new();
        atts.insert(
            "f1".to_string(),
            Attachment {
                label: "notes.txt".to_string(),
                media_type: "text/plain".to_string(),
                bytes: vec![9, 9],
            },
        );
        let segments = walk("file [notes.txt](attach:f1) end", &atts);
        assert_eq!(segments.len(), 1);
        assert_eq!(
            text_seg(&segments, 0),
            "file [attachment: notes.txt (2 bytes)] end"
        );
    }
}
