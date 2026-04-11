use crate::parse::Span;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    // -- Common errors (all artifact types) ----------------------------------
    #[error("expected H1 heading at start of file")]
    MissingH1 { span: Span },

    #[error("expected summary paragraph after H1")]
    MissingSummary { span: Span },

    #[error("summary must be a plain paragraph")]
    InvalidSummary { span: Span },

    #[error("content appears before H1 heading")]
    ContentBeforeH1 { span: Span },

    // -- Spec errors ----------------------------------------------------------
    #[error("all H2 headings in a spec must start with 'Requirement: '")]
    InvalidRequirementPrefix { span: Span },

    #[error("all H3 headings in a spec must start with 'Scenario: '")]
    InvalidScenarioPrefix { span: Span },

    #[error("headings deeper than H3 are not allowed in spec files")]
    HeadingTooDeep { span: Span },

    #[error("requirement name must not contain colons")]
    RequirementNameColon { span: Span },

    #[error("requirement '{name}' has no prose and no scenarios")]
    EmptyRequirement { name: String, span: Span },

    #[error("scenario body must be a GWT list, optionally followed by a test marker")]
    InvalidScenarioBody { span: Span },

    #[error("scenario '{name}' must contain at least one WHEN clause")]
    MissingWhen { name: String, span: Span },

    #[error("scenario '{name}' must contain at least one THEN clause")]
    MissingThen { name: String, span: Span },

    #[error("unrecognized GWT keyword in list item")]
    InvalidGwtKeyword { span: Span },

    #[error("GWT clause out of order (expected GIVEN → WHEN → THEN)")]
    GwtClauseOutOfOrder { span: Span },

    #[error("scenario '{name}' has no test marker and requirement has none to inherit")]
    UnresolvedTestMarker { name: String, span: Span },

    #[error("test marker must appear at end of its containing section")]
    MisplacedTestMarker { span: Span },

    #[error("unrecognized test marker prefix (expected test:, manual:, or skip:)")]
    InvalidTestMarker { span: Span },

    #[error("unexpected content in scenario body (only GWT list and test marker allowed)")]
    UnexpectedScenarioContent { span: Span },

    // -- Delta errors ---------------------------------------------------------
    #[error("every heading in a delta file must carry a marker (+, -, ~, =, @)")]
    MissingDeltaMarker { span: Span },

    #[error("unrecognized delta marker character")]
    InvalidDeltaMarker { span: Span },

    #[error("remove (-) entry must have an empty body")]
    NonEmptyRemoveBody { span: Span },

    #[error("rename (=) entry must contain exactly one new-name line")]
    InvalidRenameEntry { span: Span },

    #[error("anchor (@) marker is not valid on H3 headings")]
    AnchorOnH3 { span: Span },

    #[error("add (+) marker is not valid on H1 in a delta file")]
    AddOnH1 { span: Span },

    #[error("children of ~ and + entries must not carry delta markers")]
    MarkerOnContentChild { span: Span },

    // -- Step errors ----------------------------------------------------------
    #[error("step must contain a '## Tasks' section")]
    MissingTasksSection { span: Span },

    #[error("'## Tasks' must contain at least one task")]
    EmptyTasksSection { span: Span },

    #[error("unrecognized section heading in step file")]
    UnknownStepSection { name: String, span: Span },

    #[error("subtask nesting deeper than one level is not allowed")]
    SubtaskTooDeep { span: Span },

    #[error("task must have a checkbox (- [ ] or - [x])")]
    MissingCheckbox { span: Span },

    #[error("step H1 slug does not match filename slug")]
    SlugMismatch {
        expected: String,
        actual: String,
        span: Span,
    },
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
