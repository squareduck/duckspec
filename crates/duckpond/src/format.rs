//! Artifact formatting: canonical rendering with width-aware prose reflow.

pub mod prose;
pub mod table;

use std::path::{Path, PathBuf};

use crate::check::{self, CheckContext};
use crate::config::FormatConfig;
use crate::error::ParseError;
use crate::layout::{self, ArtifactKind};
use crate::parse;

/// Why `format_artifact` could not produce output for a file.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    /// The path does not match any known artifact pattern.
    #[error("path does not match any known artifact pattern: {path}")]
    UnknownArtifactType { path: PathBuf },

    /// The file does not parse cleanly as its declared artifact type.
    #[error("{path}: {} parse error(s)", errors.len())]
    Parse {
        path: PathBuf,
        errors: Vec<ParseError>,
    },
}

/// Format a single artifact to its canonical form.
///
/// `relative_path` is interpreted relative to the `duckspec/` root and is
/// used to identify the artifact type. `source` is the raw markdown.
///
/// Returns the formatted markdown on success. On parse failure or unknown
/// artifact type, returns a [`FormatError`] and the caller should leave the
/// file untouched.
pub fn format_artifact(
    relative_path: &Path,
    source: &str,
    config: &FormatConfig,
) -> Result<String, FormatError> {
    let kind =
        layout::classify(relative_path).ok_or_else(|| FormatError::UnknownArtifactType {
            path: relative_path.to_path_buf(),
        })?;

    let elements = parse::parse_elements(source);

    match kind {
        ArtifactKind::CapSpec | ArtifactKind::ChangeCapSpec => {
            let spec = parse::spec::parse_spec(&elements).map_err(|errors| FormatError::Parse {
                path: relative_path.to_path_buf(),
                errors,
            })?;
            Ok(spec.render_with(config))
        }
        ArtifactKind::CapDoc
        | ArtifactKind::ChangeCapDoc
        | ArtifactKind::Proposal
        | ArtifactKind::Design
        | ArtifactKind::Codex
        | ArtifactKind::Project => {
            let doc =
                parse::doc::parse_document(&elements).map_err(|errors| FormatError::Parse {
                    path: relative_path.to_path_buf(),
                    errors,
                })?;
            Ok(doc.render_with(config))
        }
        ArtifactKind::SpecDelta | ArtifactKind::DocDelta => {
            let delta =
                parse::delta::parse_delta(&elements).map_err(|errors| FormatError::Parse {
                    path: relative_path.to_path_buf(),
                    errors,
                })?;
            // Duplicate headings can't be auto-fixed by reordering; surface
            // them as parse errors so the file is left untouched.
            let dup_errs = check::validate_delta_duplicates(&delta);
            if !dup_errs.is_empty() {
                return Err(FormatError::Parse {
                    path: relative_path.to_path_buf(),
                    errors: dup_errs,
                });
            }
            Ok(delta.render_with(config))
        }
        ArtifactKind::Step => {
            let step = parse::step::parse_step(&elements).map_err(|errors| FormatError::Parse {
                path: relative_path.to_path_buf(),
                errors,
            })?;
            // Slug mismatch between H1 and filename is a structural problem
            // that formatting can't resolve.
            let filename = relative_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("");
            let ctx = CheckContext {
                filename_slug: layout::extract_step_slug(filename),
            };
            let mut slug_errs = Vec::new();
            check::validate_step_slug(&step, &ctx, &mut slug_errs);
            if !slug_errs.is_empty() {
                return Err(FormatError::Parse {
                    path: relative_path.to_path_buf(),
                    errors: slug_errs,
                });
            }
            Ok(step.render_with(config))
        }
    }
}

/// Format free-form markdown using the doc parse path — the same one used
/// internally by `Codex`, `Project`, `Proposal`, `Design`, and the doc-shaped
/// cap/change artifacts. Use this when the source has no canonical path under
/// `duckspec/` (e.g. duckboard ideas), so `format_artifact`'s path-based
/// classification doesn't apply.
pub fn format_doc(source: &str, config: &FormatConfig) -> Result<String, Vec<ParseError>> {
    let elements = parse::parse_elements(source);
    let doc = parse::doc::parse_document(&elements)?;
    Ok(doc.render_with(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn cfg() -> FormatConfig {
        FormatConfig::default()
    }

    #[test]
    fn unknown_artifact_type() {
        let err = format_artifact(&PathBuf::from("README.md"), "# Hello\n", &cfg()).unwrap_err();
        assert!(matches!(err, FormatError::UnknownArtifactType { .. }));
    }

    #[test]
    fn formats_a_minimal_doc() {
        let source = "# Hello\n\nA short summary.\n";
        let out =
            format_artifact(&PathBuf::from("caps/auth/doc.md"), source, &cfg()).unwrap();
        assert!(out.starts_with("# Hello\n\nA short summary."));
    }

    #[test]
    fn formats_a_minimal_spec() {
        let source = "\
# Authentication

Allows users to sign in.

## Requirement: Sign in

The system SHALL allow sign in.

### Scenario: Valid credentials

- **WHEN** the user submits credentials
- **THEN** they are signed in

> test: code
";
        let out =
            format_artifact(&PathBuf::from("caps/auth/spec.md"), source, &cfg()).unwrap();
        assert!(out.contains("# Authentication"));
        assert!(out.contains("## Requirement: Sign in"));
    }

    #[test]
    fn parse_error_is_surfaced() {
        // Spec without H1 → parse error.
        let source = "no heading\n";
        let err = format_artifact(&PathBuf::from("caps/auth/spec.md"), source, &cfg())
            .unwrap_err();
        match err {
            FormatError::Parse { path, errors } => {
                assert_eq!(path, PathBuf::from("caps/auth/spec.md"));
                assert!(!errors.is_empty());
            }
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn step_slug_mismatch_is_surfaced() {
        let source = "\
# Different name

Summary.

## Tasks

- [ ] 1. Do the thing
";
        let err = format_artifact(
            &PathBuf::from("changes/foo/steps/01-scaffold.md"),
            source,
            &cfg(),
        )
        .unwrap_err();
        match err {
            FormatError::Parse { errors, .. } => {
                assert!(errors
                    .iter()
                    .any(|e| matches!(e, ParseError::SlugMismatch { .. })));
            }
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn idempotent() {
        let source = "\
# Authentication

Allows users to sign in with email and password. Primary auth mechanism for consumer accounts.

## Requirement: Sign in

The system SHALL allow registered users to sign in.

### Scenario: Valid credentials

- **WHEN** the user submits valid email and password
- **THEN** the system issues a session token

> test: code
";
        let once =
            format_artifact(&PathBuf::from("caps/auth/spec.md"), source, &cfg()).unwrap();
        let twice =
            format_artifact(&PathBuf::from("caps/auth/spec.md"), &once, &cfg()).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn format_doc_minimal() {
        let source = "# Hello\n\nA short summary.\n";
        let out = format_doc(source, &cfg()).unwrap();
        assert!(out.starts_with("# Hello\n\nA short summary."));
    }

    #[test]
    fn format_doc_parse_error() {
        let source = "no heading\n";
        let errors = format_doc(source, &cfg()).unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn format_doc_idempotent() {
        let source = "# Idea\n\nA thought worth keeping. It has a few sentences and might wrap once the line width is reached.\n";
        let once = format_doc(source, &cfg()).unwrap();
        let twice = format_doc(&once, &cfg()).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn paragraph_that_is_a_table_gets_fenced() {
        let source = "\
# Notes

Summary.

| col1 | col2 |
|------|------|
| a    | b    |
";
        let out = format_artifact(&PathBuf::from("codex/notes.md"), source, &cfg()).unwrap();
        assert!(
            out.contains("```\n| col1 | col2 |"),
            "expected table to be wrapped in fence:\n{out}"
        );
    }
}
