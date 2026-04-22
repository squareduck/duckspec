use crate::artifact::delta::*;
use crate::artifact::doc::*;
use crate::error::MergeError;
use crate::parse::{self, Span};

/// Apply a delta to a source document, producing the merged result.
///
/// Both inputs are raw markdown strings. The source is parsed as a
/// [`Document`] (generic heading tree) and the delta is parsed as a
/// [`Delta`]. The merge operates on the heading tree structure — it is
/// content-agnostic and works for both spec and doc files.
///
/// Returns:
/// - `Ok(Some(string))` — the merged document rendered as markdown
/// - `Ok(None)` — the delta deletes the entire document (`-` on H1)
/// - `Err(errors)` — one or more merge errors occurred
///
/// After a successful merge, the caller should re-parse the result with
/// the appropriate type-specific parser (spec or doc) to validate the
/// merged content against its schema.
pub fn apply_delta(source: &str, delta: &str) -> Result<Option<String>, Vec<MergeError>> {
    // -- Parse both inputs ---------------------------------------------------
    let source_elements = parse::parse_elements(source);
    let doc = parse::doc::parse_document(&source_elements)
        .map_err(|e| vec![MergeError::SourceParseError(e)])?;

    let delta_elements = parse::parse_elements(delta);
    let delta = parse::delta::parse_delta(&delta_elements)
        .map_err(|e| vec![MergeError::DeltaParseError(e)])?;

    let mut errors = Vec::new();

    // -- H1 handling ---------------------------------------------------------
    let result = match delta.marker {
        DeltaMarker::Remove => {
            // `-` on H1: delete the entire document.
            if doc.title.trim() != delta.title.trim() {
                errors.push(MergeError::TitleMismatch {
                    delta_title: delta.title.clone(),
                    source_title: doc.title.clone(),
                    span: delta.title_span,
                });
            }
            if !errors.is_empty() {
                return Err(errors);
            }
            return Ok(None);
        }

        DeltaMarker::Replace => {
            // `~` on H1: replace entire content with delta body.
            if doc.title.trim() != delta.title.trim() {
                errors.push(MergeError::TitleMismatch {
                    delta_title: delta.title.clone(),
                    source_title: doc.title.clone(),
                    span: delta.title_span,
                });
                return Err(errors);
            }
            build_document_from_delta(&delta, &doc.title)
        }

        DeltaMarker::Rename => {
            // `=` on H1: rename the title, then process children like `@`.
            if doc.title.trim() != delta.title.trim() {
                errors.push(MergeError::TitleMismatch {
                    delta_title: delta.title.clone(),
                    source_title: doc.title.clone(),
                    span: delta.title_span,
                });
                return Err(errors);
            }
            let new_title = delta.summary.as_deref().unwrap_or(&doc.title).to_string();
            let mut result = doc.clone();
            result.title = new_title;
            apply_entries(&mut result.sections, &delta.entries, &mut errors);
            result
        }

        DeltaMarker::Anchor => {
            // `@` on H1: optionally replace summary/description, process children.
            if doc.title.trim() != delta.title.trim() {
                errors.push(MergeError::TitleMismatch {
                    delta_title: delta.title.clone(),
                    source_title: doc.title.clone(),
                    span: delta.title_span,
                });
                return Err(errors);
            }
            let mut result = doc.clone();
            if let Some(ref summary) = delta.summary {
                result.summary = summary.clone();
                result.summary_span = delta.summary_span.unwrap_or(result.summary_span);
            }
            if !delta.description.is_empty() {
                result.description = delta.description.clone();
            }
            apply_entries(&mut result.sections, &delta.entries, &mut errors);
            result
        }

        DeltaMarker::Add => {
            errors.push(MergeError::HeadingNotFound {
                marker: '+',
                name: delta.title.clone(),
                span: delta.title_span,
            });
            return Err(errors);
        }
    };

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(Some(result.render()))
}

// ---------------------------------------------------------------------------
// H2-level entry application
// ---------------------------------------------------------------------------

fn apply_entries(
    sections: &mut Vec<Section>,
    entries: &[DeltaEntry],
    errors: &mut Vec<MergeError>,
) {
    for entry in entries {
        match entry.marker {
            DeltaMarker::Rename => apply_rename_entry(sections, entry, errors),
            DeltaMarker::Remove => apply_remove_entry(sections, entry, errors),
            DeltaMarker::Replace => apply_replace_entry(sections, entry, errors),
            DeltaMarker::Anchor => apply_anchor_entry(sections, entry, errors),
            DeltaMarker::Add => apply_add_entry(sections, entry, errors),
        }
    }
}

fn apply_rename_entry(sections: &mut [Section], entry: &DeltaEntry, errors: &mut Vec<MergeError>) {
    let new_name = match &entry.rename_to {
        Some(name) => name.clone(),
        None => return,
    };

    if sections.iter().any(|s| s.heading.trim() == new_name.trim()) {
        errors.push(MergeError::RenameCollision {
            new_name: new_name.clone(),
            span: entry.heading_span,
        });
        return;
    }

    match find_section_mut(sections, &entry.heading) {
        Some(section) => {
            section.heading = new_name;
        }
        None => {
            errors.push(MergeError::HeadingNotFound {
                marker: '=',
                name: entry.heading.clone(),
                span: entry.heading_span,
            });
        }
    }
}

fn apply_remove_entry(
    #[allow(clippy::ptr_arg)] sections: &mut Vec<Section>,
    entry: &DeltaEntry,
    errors: &mut Vec<MergeError>,
) {
    match find_section_index(sections, &entry.heading) {
        Some(idx) => {
            sections.remove(idx);
        }
        None => {
            errors.push(MergeError::HeadingNotFound {
                marker: '-',
                name: entry.heading.clone(),
                span: entry.heading_span,
            });
        }
    }
}

fn apply_replace_entry(sections: &mut [Section], entry: &DeltaEntry, errors: &mut Vec<MergeError>) {
    match find_section_mut(sections, &entry.heading) {
        Some(section) => {
            section.body = entry.body.clone();
            // For ~, children are Content sections — use them directly.
            if let DeltaChildren::Content(ref content) = entry.children {
                section.children = content.clone();
            }
        }
        None => {
            errors.push(MergeError::HeadingNotFound {
                marker: '~',
                name: entry.heading.clone(),
                span: entry.heading_span,
            });
        }
    }
}

fn apply_anchor_entry(sections: &mut [Section], entry: &DeltaEntry, errors: &mut Vec<MergeError>) {
    match find_section_mut(sections, &entry.heading) {
        Some(section) => {
            if !entry.body.is_empty() {
                section.body = entry.body.clone();
            }
            // For @, children are Operations — apply them recursively.
            if let DeltaChildren::Operations(ref ops) = entry.children {
                apply_child_entries(&mut section.children, ops, section.level + 1, errors);
            }
        }
        None => {
            errors.push(MergeError::HeadingNotFound {
                marker: '@',
                name: entry.heading.clone(),
                span: entry.heading_span,
            });
        }
    }
}

fn apply_add_entry(sections: &mut Vec<Section>, entry: &DeltaEntry, errors: &mut Vec<MergeError>) {
    if find_section_index(sections, &entry.heading).is_some() {
        errors.push(MergeError::DuplicateAdd {
            name: entry.heading.clone(),
            span: entry.heading_span,
        });
        return;
    }

    let level = sections.first().map_or(2, |s| s.level);

    // For +, children are Content sections.
    let children = if let DeltaChildren::Content(ref content) = entry.children {
        content.clone()
    } else {
        vec![]
    };

    sections.push(Section {
        heading: entry.heading.clone(),
        level,
        heading_span: entry.heading_span,
        body: entry.body.clone(),
        children,
    });
}

// ---------------------------------------------------------------------------
// H3-level child entry application (under @ only)
// ---------------------------------------------------------------------------

fn apply_child_entries(
    children: &mut Vec<Section>,
    child_entries: &[DeltaChildEntry],
    child_level: u8,
    errors: &mut Vec<MergeError>,
) {
    for entry in child_entries {
        match entry.marker {
            DeltaMarker::Rename => {
                let new_name = match &entry.rename_to {
                    Some(name) => name.clone(),
                    None => continue,
                };
                if children.iter().any(|s| s.heading.trim() == new_name.trim()) {
                    errors.push(MergeError::RenameCollision {
                        new_name: new_name.clone(),
                        span: entry.heading_span,
                    });
                    continue;
                }
                match find_section_mut(children, &entry.heading) {
                    Some(section) => section.heading = new_name,
                    None => {
                        errors.push(MergeError::HeadingNotFound {
                            marker: '=',
                            name: entry.heading.clone(),
                            span: entry.heading_span,
                        });
                    }
                }
            }
            DeltaMarker::Remove => match find_section_index(children, &entry.heading) {
                Some(idx) => {
                    children.remove(idx);
                }
                None => {
                    errors.push(MergeError::HeadingNotFound {
                        marker: '-',
                        name: entry.heading.clone(),
                        span: entry.heading_span,
                    });
                }
            },
            DeltaMarker::Replace => match find_section_mut(children, &entry.heading) {
                Some(section) => {
                    section.body = entry.body.clone();
                    section.children.clear();
                }
                None => {
                    errors.push(MergeError::HeadingNotFound {
                        marker: '~',
                        name: entry.heading.clone(),
                        span: entry.heading_span,
                    });
                }
            },
            DeltaMarker::Anchor => {
                errors.push(MergeError::HeadingNotFound {
                    marker: '@',
                    name: entry.heading.clone(),
                    span: entry.heading_span,
                });
            }
            DeltaMarker::Add => {
                if find_section_index(children, &entry.heading).is_some() {
                    errors.push(MergeError::DuplicateAdd {
                        name: entry.heading.clone(),
                        span: entry.heading_span,
                    });
                    continue;
                }
                let level = children.first().map_or(child_level, |s| s.level);
                children.push(Section {
                    heading: entry.heading.clone(),
                    level,
                    heading_span: entry.heading_span,
                    body: entry.body.clone(),
                    children: vec![],
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn find_section_mut<'a>(sections: &'a mut [Section], heading: &str) -> Option<&'a mut Section> {
    sections
        .iter_mut()
        .find(|s| s.heading.trim() == heading.trim())
}

fn find_section_index(sections: &[Section], heading: &str) -> Option<usize> {
    sections
        .iter()
        .position(|s| s.heading.trim() == heading.trim())
}

/// Build a full Document from a `~` (replace) delta.
fn build_document_from_delta(delta: &Delta, source_title: &str) -> Document {
    let dummy_span = Span {
        offset: 0,
        length: 0,
    };

    let summary = delta.summary.clone().unwrap_or_default();

    let sections: Vec<Section> = delta
        .entries
        .iter()
        .map(|entry| {
            let children = match &entry.children {
                DeltaChildren::Content(secs) => secs.clone(),
                DeltaChildren::Operations(_) => vec![], // shouldn't happen for ~
            };
            Section {
                heading: entry.heading.clone(),
                level: 2,
                heading_span: entry.heading_span,
                body: entry.body.clone(),
                children,
            }
        })
        .collect();

    Document {
        title: source_title.to_string(),
        title_span: delta.title_span,
        summary,
        summary_span: delta.summary_span.unwrap_or(dummy_span),
        description: delta.description.clone(),
        sections,
    }
}
