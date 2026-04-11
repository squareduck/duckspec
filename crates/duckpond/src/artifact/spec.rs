use crate::parse::{Element, Span};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub struct Spec {
    pub title: String,
    pub title_span: Span,
    pub summary: String,
    pub summary_span: Span,
    pub description: Vec<Element>,
    pub requirements: Vec<Requirement>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Requirement {
    pub name: String,
    pub name_span: Span,
    pub prose: Vec<Element>,
    pub test_marker: Option<TestMarker>,
    pub scenarios: Vec<Scenario>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Scenario {
    pub name: String,
    pub name_span: Span,
    pub givens: Vec<Clause>,
    pub whens: Vec<Clause>,
    pub thens: Vec<Clause>,
    pub test_marker: Option<TestMarker>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Clause {
    pub text: String,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq)]
pub struct TestMarker {
    pub kind: TestMarkerKind,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq)]
pub enum TestMarkerKind {
    Code { backlinks: Vec<Backlink> },
    Manual { reason: String },
    Skip { reason: String },
}

#[derive(Clone, PartialEq, Eq)]
pub struct Backlink {
    pub path: String,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Debug impls — speaks the language of the struct, not markdown
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

fn fmt_test_marker(
    f: &mut std::fmt::Formatter<'_>,
    marker: &TestMarker,
    indent: &str,
) -> std::fmt::Result {
    match &marker.kind {
        TestMarkerKind::Code { backlinks } => {
            write!(f, "\n{indent}MARKS: test: code")?;
            for link in backlinks {
                write!(f, "\n{indent}  {}", link.path)?;
            }
        }
        TestMarkerKind::Manual { reason } => {
            write!(f, "\n{indent}MARKS: manual: {reason}")?;
        }
        TestMarkerKind::Skip { reason } => {
            write!(f, "\n{indent}MARKS: skip: {reason}")?;
        }
    }
    Ok(())
}

impl std::fmt::Debug for Spec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SPEC: {}", self.title)?;
        write!(f, "\n  SUMMARY: ")?;
        fmt_indented(f, "  ", &self.summary)?;
        if !self.description.is_empty() {
            write!(f, "\n  DESCRIPTION:")?;
            fmt_elements(f, &self.description, "    ")?;
        }
        for req in &self.requirements {
            write!(f, "\n{req:?}")?;
        }
        writeln!(f)?;
        Ok(())
    }
}

impl std::fmt::Debug for Requirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "  REQUIREMENT: {}", self.name)?;
        fmt_elements(f, &self.prose, "    ")?;
        if let Some(marker) = &self.test_marker {
            fmt_test_marker(f, marker, "    ")?;
        }
        for scenario in &self.scenarios {
            write!(f, "\n{scenario:?}")?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for Scenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "    SCENARIO: {}", self.name)?;
        if !self.givens.is_empty() {
            write!(f, "\n      GIVENS:")?;
            for clause in &self.givens {
                write!(f, "\n        ")?;
                fmt_indented(f, "        ", &clause.text)?;
            }
        }
        if !self.whens.is_empty() {
            write!(f, "\n      WHENS:")?;
            for clause in &self.whens {
                write!(f, "\n        ")?;
                fmt_indented(f, "        ", &clause.text)?;
            }
        }
        if !self.thens.is_empty() {
            write!(f, "\n      THENS:")?;
            for clause in &self.thens {
                write!(f, "\n        ")?;
                fmt_indented(f, "        ", &clause.text)?;
            }
        }
        if let Some(marker) = &self.test_marker {
            fmt_test_marker(f, marker, "      ")?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for TestMarker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            TestMarkerKind::Code { backlinks } => {
                write!(f, "test: code")?;
                for link in backlinks {
                    write!(f, "\n  {}", link.path)?;
                }
            }
            TestMarkerKind::Manual { reason } => write!(f, "manual: {reason}")?,
            TestMarkerKind::Skip { reason } => write!(f, "skip: {reason}")?,
        }
        Ok(())
    }
}

impl std::fmt::Debug for TestMarkerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestMarkerKind::Code { backlinks } => {
                write!(f, "test: code")?;
                for link in backlinks {
                    write!(f, "\n  {}", link.path)?;
                }
            }
            TestMarkerKind::Manual { reason } => write!(f, "manual: {reason}")?,
            TestMarkerKind::Skip { reason } => write!(f, "skip: {reason}")?,
        }
        Ok(())
    }
}

impl std::fmt::Debug for Clause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl std::fmt::Debug for Backlink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}
