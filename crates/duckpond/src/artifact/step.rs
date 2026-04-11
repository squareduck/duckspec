use crate::parse::{Element, Span};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub struct Step {
    pub title: String,
    pub title_span: Span,
    pub slug: String,
    pub summary: String,
    pub summary_span: Span,
    pub description: Vec<Element>,
    pub prerequisites: Option<Vec<Prerequisite>>,
    pub context: Option<Vec<Element>>,
    pub tasks: Vec<Task>,
    pub outcomes: Option<Vec<Element>>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Prerequisite {
    pub kind: PrerequisiteKind,
    pub checked: bool,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq)]
pub enum PrerequisiteKind {
    StepRef { slug: String },
    Freeform { text: String },
}

#[derive(Clone, PartialEq, Eq)]
pub struct Task {
    pub content: TaskContent,
    pub checked: bool,
    pub span: Span,
    pub subtasks: Vec<Subtask>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Subtask {
    pub content: TaskContent,
    pub checked: bool,
    pub span: Span,
}

#[derive(Clone, PartialEq, Eq)]
pub enum TaskContent {
    Freeform {
        text: String,
    },
    SpecRef {
        capability: String,
        requirement: String,
        scenario: String,
    },
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a step title to a slug (kebab-case).
pub fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
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

impl std::fmt::Debug for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "STEP: {} (slug: {})", self.title, self.slug)?;
        write!(f, "\n  SUMMARY: ")?;
        fmt_indented(f, "  ", &self.summary)?;
        if !self.description.is_empty() {
            write!(f, "\n  DESCRIPTION:")?;
            fmt_elements(f, &self.description, "    ")?;
        }
        if let Some(prereqs) = &self.prerequisites {
            write!(f, "\n  PREREQUISITES:")?;
            for prereq in prereqs {
                write!(f, "\n    {prereq:?}")?;
            }
        }
        if let Some(ctx) = &self.context {
            write!(f, "\n  CONTEXT:")?;
            fmt_elements(f, ctx, "    ")?;
        }
        write!(f, "\n  TASKS:")?;
        for task in &self.tasks {
            write!(f, "\n{task:?}")?;
        }
        if let Some(outcomes) = &self.outcomes {
            write!(f, "\n  OUTCOMES:")?;
            fmt_elements(f, outcomes, "    ")?;
        }
        writeln!(f)?;
        Ok(())
    }
}

impl std::fmt::Debug for Prerequisite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let check = if self.checked { "x" } else { " " };
        match &self.kind {
            PrerequisiteKind::StepRef { slug } => write!(f, "[{check}] @step {slug}"),
            PrerequisiteKind::Freeform { text } => {
                write!(f, "[{check}] ")?;
                fmt_indented(f, "    ", text)
            }
        }
    }
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let check = if self.checked { "x" } else { " " };
        write!(f, "    [{check}] {:?}", self.content)?;
        for sub in &self.subtasks {
            write!(f, "\n      {sub:?}")?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for Subtask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let check = if self.checked { "x" } else { " " };
        write!(f, "[{check}] {:?}", self.content)
    }
}

impl std::fmt::Debug for TaskContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskContent::Freeform { text } => write!(f, "{text}"),
            TaskContent::SpecRef {
                capability,
                requirement,
                scenario,
            } => write!(f, "@spec {capability} {requirement}: {scenario}"),
        }
    }
}
