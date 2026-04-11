use crate::artifact::doc::Section;
use crate::parse::{Element, Span};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaMarker {
    Add,     // +
    Remove,  // -
    Replace, // ~
    Rename,  // =
    Anchor,  // @
}

impl DeltaMarker {
    /// Canonical sort order: = - ~ @ +
    pub fn order(&self) -> u8 {
        match self {
            DeltaMarker::Rename => 0,
            DeltaMarker::Remove => 1,
            DeltaMarker::Replace => 2,
            DeltaMarker::Anchor => 3,
            DeltaMarker::Add => 4,
        }
    }

    pub fn char(&self) -> char {
        match self {
            DeltaMarker::Add => '+',
            DeltaMarker::Remove => '-',
            DeltaMarker::Replace => '~',
            DeltaMarker::Rename => '=',
            DeltaMarker::Anchor => '@',
        }
    }

    pub fn from_char(c: char) -> Option<DeltaMarker> {
        match c {
            '+' => Some(DeltaMarker::Add),
            '-' => Some(DeltaMarker::Remove),
            '~' => Some(DeltaMarker::Replace),
            '=' => Some(DeltaMarker::Rename),
            '@' => Some(DeltaMarker::Anchor),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Delta {
    pub marker: DeltaMarker,
    pub title: String,
    pub title_span: Span,
    pub summary: Option<String>,
    pub summary_span: Option<Span>,
    pub description: Vec<Element>,
    pub entries: Vec<DeltaEntry>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct DeltaEntry {
    pub marker: DeltaMarker,
    pub heading: String,
    pub heading_span: Span,
    pub body: Vec<Element>,
    pub rename_to: Option<String>,
    pub children: DeltaChildren,
}

/// Children of a delta entry.
///
/// - Under `@` (anchor): children are delta operations with markers.
/// - Under `~` (replace) and `+` (add): children are plain content
///   sections — markers are forbidden.
/// - Under `=` (rename) and `-` (remove): no children allowed.
#[derive(Clone, PartialEq, Eq)]
pub enum DeltaChildren {
    /// Children are delta operations with markers (under `@` entries).
    Operations(Vec<DeltaChildEntry>),
    /// Children are plain content sections (under `~` and `+` entries).
    Content(Vec<Section>),
}

impl DeltaChildren {
    pub fn is_empty(&self) -> bool {
        match self {
            DeltaChildren::Operations(ops) => ops.is_empty(),
            DeltaChildren::Content(secs) => secs.is_empty(),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DeltaChildEntry {
    pub marker: DeltaMarker,
    pub heading: String,
    pub heading_span: Span,
    pub body: Vec<Element>,
    pub rename_to: Option<String>,
}

// ---------------------------------------------------------------------------
// Debug impls
// ---------------------------------------------------------------------------

fn fmt_indented(f: &mut std::fmt::Formatter<'_>, indent: &str, text: &str) -> std::fmt::Result {
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            write!(f, "\n{indent}  {line}")?;
        } else {
            write!(f, "{line}")?;
        }
    }
    Ok(())
}

fn fmt_elements(
    f: &mut std::fmt::Formatter<'_>,
    elements: &[Element],
    indent: &str,
) -> std::fmt::Result {
    for elem in elements {
        write!(f, "\n{indent}")?;
        fmt_indented(f, indent, &format!("{elem:?}"))?;
    }
    Ok(())
}

impl std::fmt::Debug for Delta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DELTA: {} {}", self.marker.char(), self.title)?;
        if let Some(summary) = &self.summary {
            write!(f, "\n  SUMMARY: ")?;
            fmt_indented(f, "  ", summary)?;
        }
        if !self.description.is_empty() {
            write!(f, "\n  DESCRIPTION:")?;
            fmt_elements(f, &self.description, "    ")?;
        }
        for entry in &self.entries {
            write!(f, "\n{entry:?}")?;
        }
        writeln!(f)?;
        Ok(())
    }
}

impl std::fmt::Debug for DeltaEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "  {} ENTRY: {}", self.marker.char(), self.heading)?;
        if let Some(new_name) = &self.rename_to {
            write!(f, "\n    RENAME_TO: {new_name}")?;
        }
        fmt_elements(f, &self.body, "    ")?;
        match &self.children {
            DeltaChildren::Operations(ops) => {
                for child in ops {
                    write!(f, "\n{child:?}")?;
                }
            }
            DeltaChildren::Content(sections) => {
                for section in sections {
                    write!(f, "\n    CONTENT: {}", section.heading)?;
                    fmt_elements(f, &section.body, "      ")?;
                }
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for DeltaChildEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "    {} CHILD: {}", self.marker.char(), self.heading)?;
        if let Some(new_name) = &self.rename_to {
            write!(f, "\n      RENAME_TO: {new_name}")?;
        }
        fmt_elements(f, &self.body, "      ")?;
        Ok(())
    }
}
