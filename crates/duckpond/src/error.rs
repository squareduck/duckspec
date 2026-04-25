use std::path::PathBuf;

use miette::Diagnostic;

use crate::parse::Span;

#[derive(Debug, Clone, thiserror::Error, Diagnostic)]
pub enum ParseError {
    // -- Common errors (all artifact types) ----------------------------------
    #[error("expected H1 heading at start of file")]
    MissingH1 {
        #[label("here")]
        span: Span,
    },

    #[error("expected summary paragraph after H1")]
    MissingSummary {
        #[label("here")]
        span: Span,
    },

    #[error("summary must be a plain paragraph")]
    InvalidSummary {
        #[label("not a plain paragraph")]
        span: Span,
    },

    #[error("content appears before H1 heading")]
    ContentBeforeH1 {
        #[label("unexpected content")]
        span: Span,
    },

    // -- Spec errors ----------------------------------------------------------
    #[error("all H2 headings in a spec must start with 'Requirement: '")]
    InvalidRequirementPrefix {
        #[label("expected 'Requirement: ' prefix")]
        span: Span,
    },

    #[error("all H3 headings in a spec must start with 'Scenario: '")]
    InvalidScenarioPrefix {
        #[label("expected 'Scenario: ' prefix")]
        span: Span,
    },

    #[error("headings deeper than H3 are not allowed in spec files")]
    HeadingTooDeep {
        #[label("too deep")]
        span: Span,
    },

    #[error("requirement name must not contain colons")]
    RequirementNameColon {
        #[label("colon in name")]
        span: Span,
    },

    #[error("requirement '{name}' has no prose and no scenarios")]
    EmptyRequirement {
        name: String,
        #[label("empty requirement")]
        span: Span,
    },

    #[error("scenario body must be a GWT list, optionally followed by a test marker")]
    InvalidScenarioBody {
        #[label("invalid body")]
        span: Span,
    },

    #[error("scenario '{name}' must contain at least one WHEN clause")]
    MissingWhen {
        name: String,
        #[label("missing WHEN")]
        span: Span,
    },

    #[error("scenario '{name}' must contain at least one THEN clause")]
    MissingThen {
        name: String,
        #[label("missing THEN")]
        span: Span,
    },

    #[error("unrecognized GWT keyword in list item")]
    InvalidGwtKeyword {
        #[label("unrecognized keyword")]
        span: Span,
    },

    #[error("GWT clause out of order (expected GIVEN → WHEN → THEN)")]
    GwtClauseOutOfOrder {
        #[label("out of order")]
        span: Span,
    },

    #[error("scenario '{name}' has no test marker and requirement has none to inherit")]
    UnresolvedTestMarker {
        name: String,
        #[label("no test marker")]
        span: Span,
    },

    #[error("test marker must appear at end of its containing section")]
    MisplacedTestMarker {
        #[label("misplaced")]
        span: Span,
    },

    #[error("unrecognized test marker prefix (expected test:, manual:, or skip:)")]
    InvalidTestMarker {
        #[label("unrecognized prefix")]
        span: Span,
    },

    #[error("unexpected content in scenario body (only GWT list and test marker allowed)")]
    UnexpectedScenarioContent {
        #[label("unexpected content")]
        span: Span,
    },

    // -- Delta errors ---------------------------------------------------------
    #[error("every heading in a delta file must carry a marker (+, -, ~, =, @)")]
    MissingDeltaMarker {
        #[label("missing marker")]
        span: Span,
    },

    #[error("delta marker must be followed by a space")]
    MarkerMissingSpace {
        #[label("missing space after marker")]
        span: Span,
    },

    #[error("unrecognized delta marker character")]
    InvalidDeltaMarker {
        #[label("unrecognized marker")]
        span: Span,
    },

    #[error("remove (-) entry must have an empty body")]
    NonEmptyRemoveBody {
        #[label("non-empty body")]
        span: Span,
    },

    #[error("rename (=) entry must contain exactly one new-name line")]
    InvalidRenameEntry {
        #[label("invalid rename")]
        span: Span,
    },

    #[error("anchor (@) marker is not valid on H3 headings")]
    AnchorOnH3 {
        #[label("@ not valid here")]
        span: Span,
    },

    #[error("add (+) marker is not valid on H1 in a delta file")]
    AddOnH1 {
        #[label("+ not valid on H1")]
        span: Span,
    },

    #[error("children of ~ and + entries must not carry delta markers")]
    MarkerOnContentChild {
        #[label("marker not allowed")]
        span: Span,
    },

    #[error("duplicate delta entry heading '{name}' at this level")]
    DuplicateDeltaHeading {
        name: String,
        #[label("duplicate")]
        span: Span,
    },

    #[error("delta entries are not in canonical order (expected = - ~ @ +)")]
    DeltaOrderViolation {
        #[label("out of order")]
        span: Span,
    },

    // -- Step errors ----------------------------------------------------------
    #[error("step must contain a '## Tasks' section")]
    MissingTasksSection {
        #[label("here")]
        span: Span,
    },

    #[error("'## Tasks' must contain at least one task")]
    EmptyTasksSection {
        #[label("empty")]
        span: Span,
    },

    #[error("unrecognized section heading in step file")]
    UnknownStepSection {
        name: String,
        #[label("unrecognized")]
        span: Span,
    },

    #[error("subtask nesting deeper than one level is not allowed")]
    SubtaskTooDeep {
        #[label("too deep")]
        span: Span,
    },

    #[error("task must have a checkbox (- [ ] or - [x])")]
    MissingCheckbox {
        #[label("missing checkbox")]
        span: Span,
    },

    #[error("step H1 slug does not match filename slug")]
    SlugMismatch {
        expected: String,
        actual: String,
        #[label("expected '{expected}'")]
        span: Span,
    },
}

impl ParseError {
    /// The source span for this error.
    pub fn span(&self) -> Span {
        match self {
            ParseError::MissingH1 { span }
            | ParseError::MissingSummary { span }
            | ParseError::InvalidSummary { span }
            | ParseError::ContentBeforeH1 { span }
            | ParseError::InvalidRequirementPrefix { span }
            | ParseError::InvalidScenarioPrefix { span }
            | ParseError::HeadingTooDeep { span }
            | ParseError::RequirementNameColon { span }
            | ParseError::EmptyRequirement { span, .. }
            | ParseError::InvalidScenarioBody { span }
            | ParseError::MissingWhen { span, .. }
            | ParseError::MissingThen { span, .. }
            | ParseError::InvalidGwtKeyword { span }
            | ParseError::GwtClauseOutOfOrder { span }
            | ParseError::UnresolvedTestMarker { span, .. }
            | ParseError::MisplacedTestMarker { span }
            | ParseError::InvalidTestMarker { span }
            | ParseError::UnexpectedScenarioContent { span }
            | ParseError::MissingDeltaMarker { span }
            | ParseError::MarkerMissingSpace { span }
            | ParseError::InvalidDeltaMarker { span }
            | ParseError::NonEmptyRemoveBody { span }
            | ParseError::InvalidRenameEntry { span }
            | ParseError::AnchorOnH3 { span }
            | ParseError::AddOnH1 { span }
            | ParseError::MarkerOnContentChild { span }
            | ParseError::DuplicateDeltaHeading { span, .. }
            | ParseError::DeltaOrderViolation { span }
            | ParseError::MissingTasksSection { span }
            | ParseError::EmptyTasksSection { span }
            | ParseError::UnknownStepSection { span, .. }
            | ParseError::SubtaskTooDeep { span }
            | ParseError::MissingCheckbox { span }
            | ParseError::SlugMismatch { span, .. } => *span,
        }
    }
}

// ---------------------------------------------------------------------------
// Merge errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum MergeError {
    #[error("failed to parse source document")]
    SourceParseError(Vec<ParseError>),

    #[error("failed to parse delta")]
    DeltaParseError(Vec<ParseError>),

    #[error("H1 title mismatch: delta targets '{delta_title}' but source has '{source_title}'")]
    TitleMismatch {
        delta_title: String,
        source_title: String,
        span: Span,
    },

    #[error("{marker} target '{name}' not found in source")]
    HeadingNotFound {
        marker: char,
        name: String,
        span: Span,
    },

    #[error("cannot add '{name}': heading already exists at this level")]
    DuplicateAdd { name: String, span: Span },

    #[error("rename target '{new_name}' collides with existing heading")]
    RenameCollision { new_name: String, span: Span },
}

// ---------------------------------------------------------------------------
// Change-level errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ChangeError {
    #[error("delta '{delta_path}' targets nonexistent '{expected_path}'")]
    DeltaTargetMissing {
        delta_path: PathBuf,
        expected_path: PathBuf,
    },

    #[error("both full file and delta for '{cap_path}': {full_file} and {delta_file}")]
    FullAndDeltaConflict {
        cap_path: String,
        full_file: PathBuf,
        delta_file: PathBuf,
    },

    #[error("@step '{slug}' not found in change")]
    StepPrerequisiteNotFound { slug: String, step_file: PathBuf },

    #[error("change must not contain {kind} entries: {path}")]
    ForbiddenEntry { kind: String, path: PathBuf },

    #[error("path segment contains whitespace: '{segment}' in {path}")]
    WhitespaceInPath { segment: String, path: PathBuf },
}
