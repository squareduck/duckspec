//! Review artifact recognition and validation.
//!
//! A review is recognized purely by its path (`changes/<name>/reviews/NN-<slug>.md`)
//! and validates against the document schema, in both active and archived changes.

use std::path::Path;

use duckpond::check::{check_artifact, CheckContext};
use duckpond::layout::{classify, ArtifactKind};

// @spec review Review recognition and validation: A well-formed review validates
#[test]
fn well_formed_review_validates() {
    let path = Path::new("changes/add-oauth/reviews/01-post-implementation.md");
    let kind = classify(path).expect("review path should classify");
    assert_eq!(kind, ArtifactKind::Review);

    let source = "# Post-implementation review\n\nA short summary of the findings.\n";
    let result = check_artifact(source, &kind, &CheckContext::default());
    assert!(
        result.errors.is_empty(),
        "well-formed review should validate, got {:?}",
        result.errors
    );
}

// @spec review Review recognition and validation: A review missing its H1 title is reported as a document error
#[test]
fn review_missing_h1_is_a_document_error() {
    let path = Path::new("changes/add-oauth/reviews/01-post-implementation.md");
    let kind = classify(path).expect("review path should classify");
    assert_eq!(kind, ArtifactKind::Review);

    // No H1 title — just a body paragraph.
    let source = "A review body with no heading at all.\n";
    let result = check_artifact(source, &kind, &CheckContext::default());
    assert!(
        !result.errors.is_empty(),
        "a review without an H1 title should fail document-schema validation"
    );
}

// @spec review Review recognition and validation: A review in an archived change is still recognized
#[test]
fn archived_review_is_recognized() {
    let path = Path::new("archive/2026-03-15-01-add-oauth/reviews/01-post-implementation.md");
    assert_eq!(classify(path), Some(ArtifactKind::Review));
}
