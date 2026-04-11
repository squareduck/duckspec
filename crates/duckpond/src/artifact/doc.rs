use crate::parse::{Element, Span};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub struct Document {
    pub title: String,
    pub title_span: Span,
    pub summary: String,
    pub summary_span: Span,
    pub description: Vec<Element>,
    pub sections: Vec<Section>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Section {
    pub heading: String,
    pub level: u8,
    pub heading_span: Span,
    pub body: Vec<Element>,
    pub children: Vec<Section>,
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

impl std::fmt::Debug for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DOCUMENT: {}", self.title)?;
        write!(f, "\n  SUMMARY: ")?;
        fmt_indented(f, "  ", &self.summary)?;
        if !self.description.is_empty() {
            write!(f, "\n  DESCRIPTION:")?;
            fmt_elements(f, &self.description, "    ")?;
        }
        for section in &self.sections {
            fmt_section(f, section, 1)?;
        }
        writeln!(f)?;
        Ok(())
    }
}

impl std::fmt::Debug for Section {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt_section(f, self, 0)
    }
}

fn fmt_section(
    f: &mut std::fmt::Formatter<'_>,
    section: &Section,
    depth: usize,
) -> std::fmt::Result {
    let indent = "  ".repeat(depth);
    write!(f, "\n{indent}  SECTION: {}", section.heading)?;
    let body_indent = format!("{indent}    ");
    fmt_elements(f, &section.body, &body_indent)?;
    for child in &section.children {
        fmt_section(f, child, depth + 1)?;
    }
    Ok(())
}
