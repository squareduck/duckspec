use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::artifact::delta::{Delta, DeltaChildren, DeltaMarker};
use crate::artifact::step::PrerequisiteKind;
use crate::error::{ChangeError, ParseError};
use crate::layout::ArtifactKind;
use crate::parse::{self, Element};

// ---------------------------------------------------------------------------
// Single-file check
// ---------------------------------------------------------------------------

/// Optional metadata about the file being checked, beyond its content.
#[derive(Debug, Default)]
pub struct CheckContext {
    /// For Step artifacts: the slug extracted from the filename (e.g.
    /// `"scaffold"` from `"01-scaffold.md"`), for comparison with the
    /// slug derived from the H1 title.
    pub filename_slug: Option<String>,
}

/// Result of checking a single artifact.
#[derive(Debug)]
pub struct CheckResult {
    /// Parse errors found in the artifact.
    pub errors: Vec<ParseError>,
}

/// Validate a single artifact's content against its schema.
///
/// Dispatches to the appropriate parser based on the artifact kind.
/// Use `ctx` to supply optional metadata (e.g. filename slug for steps).
pub fn check_artifact(source: &str, kind: &ArtifactKind, ctx: &CheckContext) -> CheckResult {
    let elements = parse::parse_elements(source);

    let mut errors = match kind {
        ArtifactKind::CapSpec | ArtifactKind::ChangeCapSpec => {
            parse::spec::parse_spec(&elements).err().unwrap_or_default()
        }
        ArtifactKind::CapDoc
        | ArtifactKind::ChangeCapDoc
        | ArtifactKind::Proposal
        | ArtifactKind::Design
        | ArtifactKind::Codex
        | ArtifactKind::Project => parse::doc::parse_document(&elements)
            .err()
            .unwrap_or_default(),
        ArtifactKind::SpecDelta | ArtifactKind::DocDelta => {
            let mut errs = validate_delta_ordering(&elements);
            match parse::delta::parse_delta(&elements) {
                Ok(delta) => {
                    errs.extend(validate_delta_duplicates(&delta));
                    errs
                }
                Err(parse_errs) => {
                    errs.extend(parse_errs);
                    errs
                }
            }
        }
        ArtifactKind::Step => match parse::step::parse_step(&elements) {
            Ok(step) => {
                let mut errs = Vec::new();
                validate_step_slug(&step, ctx, &mut errs);
                errs
            }
            Err(e) => e,
        },
    };

    // Stable sort so ordering errors come before duplicate errors, etc.
    errors.sort_by_key(|e| e.span().offset);

    CheckResult { errors }
}

// ---------------------------------------------------------------------------
// Single-file validation helpers
// ---------------------------------------------------------------------------

/// Compare the step's slug (derived from H1) against the filename slug.
pub(crate) fn validate_step_slug(
    step: &crate::artifact::step::Step,
    ctx: &CheckContext,
    errors: &mut Vec<ParseError>,
) {
    if let Some(ref expected) = ctx.filename_slug
        && step.slug != *expected
    {
        errors.push(ParseError::SlugMismatch {
            expected: expected.clone(),
            actual: step.slug.clone(),
            span: step.title_span,
        });
    }
}

/// Scan raw elements for H2 delta markers and check canonical ordering.
///
/// Canonical order is `=` (0) → `-` (1) → `~` (2) → `@` (3) → `+` (4).
/// This must run on the elements *before* the parser re-sorts them.
fn validate_delta_ordering(elements: &[Element]) -> Vec<ParseError> {
    let mut errors = Vec::new();

    // Collect (marker_order, span) for each H2 heading.
    let h2_markers: Vec<(u8, parse::Span)> = elements
        .iter()
        .filter_map(|e| match e {
            Element::Heading {
                level: 2,
                content,
                span,
            } => extract_marker_order(content).map(|order| (order, *span)),
            _ => None,
        })
        .collect();

    check_order(&h2_markers, &mut errors);

    // For each @ entry at H2 level, check its H3 children.
    // Walk elements: when we see an H2 with @, collect subsequent H3s until next H2.
    let mut i = 0;
    while i < elements.len() {
        if let Element::Heading {
            level: 2, content, ..
        } = &elements[i]
            && content.trim().starts_with('@')
        {
            // Collect H3 children of this @ entry.
            let mut h3_markers = Vec::new();
            let mut j = i + 1;
            while j < elements.len() {
                match &elements[j] {
                    Element::Heading { level: 2, .. } => break,
                    Element::Heading {
                        level: 3,
                        content,
                        span,
                    } => {
                        if let Some(order) = extract_marker_order(content) {
                            h3_markers.push((order, *span));
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            check_order(&h3_markers, &mut errors);
        }
        i += 1;
    }

    errors
}

/// Extract the canonical sort order from a heading's marker character.
fn extract_marker_order(heading: &str) -> Option<u8> {
    let first = heading.trim().chars().next()?;
    DeltaMarker::from_char(first).map(|m| m.order())
}

/// Check that a sequence of (order, span) is non-decreasing.
fn check_order(items: &[(u8, parse::Span)], errors: &mut Vec<ParseError>) {
    for window in items.windows(2) {
        if window[0].0 > window[1].0 {
            errors.push(ParseError::DeltaOrderViolation { span: window[1].1 });
        }
    }
}

/// Check for duplicate heading names among delta entries and their children.
pub(crate) fn validate_delta_duplicates(delta: &Delta) -> Vec<ParseError> {
    let mut errors = Vec::new();

    // Check H2-level duplicates.
    let mut seen: HashMap<&str, parse::Span> = HashMap::new();
    for entry in &delta.entries {
        if let Some(_prev_span) = seen.get(entry.heading.as_str()) {
            errors.push(ParseError::DuplicateDeltaHeading {
                name: entry.heading.clone(),
                span: entry.heading_span,
            });
        } else {
            seen.insert(&entry.heading, entry.heading_span);
        }
    }

    // Check H3-level duplicates within each @ entry.
    for entry in &delta.entries {
        if let DeltaChildren::Operations(ref ops) = entry.children {
            let mut child_seen: HashMap<&str, parse::Span> = HashMap::new();
            for child in ops {
                if let Some(_prev_span) = child_seen.get(child.heading.as_str()) {
                    errors.push(ParseError::DuplicateDeltaHeading {
                        name: child.heading.clone(),
                        span: child.heading_span,
                    });
                } else {
                    child_seen.insert(&child.heading, child.heading_span);
                }
            }
        }
    }

    errors
}

// ---------------------------------------------------------------------------
// Change-level check
// ---------------------------------------------------------------------------

/// A file within a change, loaded by the CLI.
pub struct LoadedFile {
    /// Path relative to duckspec root (e.g. `changes/add-2fa/caps/auth/spec.delta.md`).
    pub relative_path: PathBuf,
    /// Classified artifact kind.
    pub kind: ArtifactKind,
    /// Raw markdown content.
    pub content: String,
}

/// Top-level duckspec state needed for cross-checking a change.
pub struct DuckspecState {
    /// Capability paths that have a `spec.md` at the top level.
    /// Each entry is the capability path only (e.g. `auth/oauth`, not `caps/auth/oauth/spec.md`).
    pub cap_spec_paths: HashSet<PathBuf>,
    /// Capability paths that have a `doc.md` at the top level.
    pub cap_doc_paths: HashSet<PathBuf>,
}

/// Result of checking a full change directory.
#[derive(Debug)]
pub struct ChangeCheckResult {
    /// Per-file schema errors (path relative to duckspec root → errors).
    pub file_errors: Vec<(PathBuf, Vec<ParseError>)>,
    /// Cross-file structural errors.
    pub change_errors: Vec<ChangeError>,
}

/// Validate an entire change: all files individually plus cross-file constraints.
pub fn check_change(
    change_name: &str,
    files: &[LoadedFile],
    state: &DuckspecState,
) -> ChangeCheckResult {
    let mut file_errors = Vec::new();
    let mut change_errors = Vec::new();

    // -- Per-file checks + data collection for cross-checks -------------------

    let change_prefix = PathBuf::from(format!("changes/{change_name}"));

    // Track cap paths per artifact type for exclusivity check.
    let mut spec_full_paths: HashMap<PathBuf, PathBuf> = HashMap::new();
    let mut spec_delta_paths: HashMap<PathBuf, PathBuf> = HashMap::new();
    let mut doc_full_paths: HashMap<PathBuf, PathBuf> = HashMap::new();
    let mut doc_delta_paths: HashMap<PathBuf, PathBuf> = HashMap::new();

    // Collect step slugs for prerequisite resolution.
    let mut step_slugs: HashSet<String> = HashSet::new();

    // Collect parsed steps for prerequisite checking (step file path → prereqs).
    struct StepInfo {
        file_path: PathBuf,
        prereq_slugs: Vec<String>,
    }
    let mut step_infos: Vec<StepInfo> = Vec::new();

    for file in files {
        // Build CheckContext for steps.
        let ctx = if file.kind == ArtifactKind::Step {
            let filename = file
                .relative_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("");
            CheckContext {
                filename_slug: crate::layout::extract_step_slug(filename),
            }
        } else {
            CheckContext::default()
        };

        let result = check_artifact(&file.content, &file.kind, &ctx);
        if !result.errors.is_empty() {
            file_errors.push((file.relative_path.clone(), result.errors));
        }

        // Extract capability path from the file's relative path.
        let within_change = file
            .relative_path
            .strip_prefix(&change_prefix)
            .unwrap_or(&file.relative_path);

        match file.kind {
            ArtifactKind::ChangeCapSpec => {
                if let Some(cap_path) = extract_cap_path(within_change) {
                    spec_full_paths.insert(cap_path, file.relative_path.clone());
                }
            }
            ArtifactKind::SpecDelta => {
                if let Some(cap_path) = extract_cap_path(within_change) {
                    spec_delta_paths.insert(cap_path, file.relative_path.clone());
                }
            }
            ArtifactKind::ChangeCapDoc => {
                if let Some(cap_path) = extract_cap_path(within_change) {
                    doc_full_paths.insert(cap_path, file.relative_path.clone());
                }
            }
            ArtifactKind::DocDelta => {
                if let Some(cap_path) = extract_cap_path(within_change) {
                    doc_delta_paths.insert(cap_path, file.relative_path.clone());
                }
            }
            ArtifactKind::Step => {
                // Parse step to collect slug and prerequisites.
                let elements = parse::parse_elements(&file.content);
                if let Ok(step) = parse::step::parse_step(&elements) {
                    step_slugs.insert(step.slug.clone());
                    let prereq_slugs: Vec<String> = step
                        .prerequisites
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|p| match &p.kind {
                            PrerequisiteKind::StepRef { slug } => Some(slug.clone()),
                            _ => None,
                        })
                        .collect();
                    step_infos.push(StepInfo {
                        file_path: file.relative_path.clone(),
                        prereq_slugs,
                    });
                }
            }
            _ => {}
        }
    }

    // -- Cross-file checks ----------------------------------------------------

    // 1. Path whitespace in change name.
    for segment in change_name.split('/') {
        if segment.chars().any(|c| c.is_whitespace()) {
            change_errors.push(ChangeError::WhitespaceInPath {
                segment: segment.to_string(),
                path: change_prefix.clone(),
            });
        }
    }

    // Path whitespace in capability paths within the change.
    for cap_path in spec_full_paths
        .keys()
        .chain(spec_delta_paths.keys())
        .chain(doc_full_paths.keys())
        .chain(doc_delta_paths.keys())
    {
        for segment in cap_path.iter() {
            let s = segment.to_str().unwrap_or("");
            if s.chars().any(|c| c.is_whitespace()) {
                change_errors.push(ChangeError::WhitespaceInPath {
                    segment: s.to_string(),
                    path: cap_path.clone(),
                });
            }
        }
    }

    // 2. Forbidden entries (codex/ or project.md inside a change).
    for file in files {
        let within_change = file
            .relative_path
            .strip_prefix(&change_prefix)
            .unwrap_or(&file.relative_path);
        let first_component = within_change
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str());
        if first_component == Some("codex") {
            change_errors.push(ChangeError::ForbiddenEntry {
                kind: "codex".to_string(),
                path: file.relative_path.clone(),
            });
        }
        if within_change == std::path::Path::new("project.md") {
            change_errors.push(ChangeError::ForbiddenEntry {
                kind: "project.md".to_string(),
                path: file.relative_path.clone(),
            });
        }
    }

    // 3. Full-file / delta exclusivity.
    for (cap_path, full_file) in &spec_full_paths {
        if let Some(delta_file) = spec_delta_paths.get(cap_path) {
            change_errors.push(ChangeError::FullAndDeltaConflict {
                cap_path: cap_path.display().to_string(),
                full_file: full_file.clone(),
                delta_file: delta_file.clone(),
            });
        }
    }
    for (cap_path, full_file) in &doc_full_paths {
        if let Some(delta_file) = doc_delta_paths.get(cap_path) {
            change_errors.push(ChangeError::FullAndDeltaConflict {
                cap_path: cap_path.display().to_string(),
                full_file: full_file.clone(),
                delta_file: delta_file.clone(),
            });
        }
    }

    // 4. Delta target existence.
    for (cap_path, delta_file) in &spec_delta_paths {
        if !state.cap_spec_paths.contains(cap_path) {
            change_errors.push(ChangeError::DeltaTargetMissing {
                delta_path: delta_file.clone(),
                expected_path: PathBuf::from("caps").join(cap_path).join("spec.md"),
            });
        }
    }
    for (cap_path, delta_file) in &doc_delta_paths {
        if !state.cap_doc_paths.contains(cap_path) {
            change_errors.push(ChangeError::DeltaTargetMissing {
                delta_path: delta_file.clone(),
                expected_path: PathBuf::from("caps").join(cap_path).join("doc.md"),
            });
        }
    }

    // 5. Step prerequisite resolution.
    for info in &step_infos {
        for slug in &info.prereq_slugs {
            if !step_slugs.contains(slug) {
                change_errors.push(ChangeError::StepPrerequisiteNotFound {
                    slug: slug.clone(),
                    step_file: info.file_path.clone(),
                });
            }
        }
    }

    ChangeCheckResult {
        file_errors,
        change_errors,
    }
}

/// Extract the capability path from a path within a change's `caps/` subtree.
///
/// Given `caps/auth/oauth/spec.md`, returns `auth/oauth`.
/// Given `caps/auth/spec.delta.md`, returns `auth`.
fn extract_cap_path(within_change: &std::path::Path) -> Option<PathBuf> {
    let components: Vec<&str> = within_change
        .components()
        .map(|c| c.as_os_str().to_str().unwrap_or(""))
        .collect();

    if components.first() != Some(&"caps") || components.len() < 3 {
        return None;
    }

    // Everything between "caps/" and the filename is the capability path.
    let cap_segments = &components[1..components.len() - 1];
    Some(PathBuf::from(cap_segments.join("/")))
}
