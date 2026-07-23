mod common;

use duckpond::merge::{
    MergeValidateError, Merged, apply_delta, merge_doc_delta, merge_spec_delta, summarize_errors,
};

fn merge_fixture(name: &str) -> Result<Option<String>, Vec<duckpond::error::MergeError>> {
    let source = common::load_fixture("merge", &format!("{name}_source.md"));
    let delta = common::load_fixture("merge", &format!("{name}_delta.md"));
    apply_delta(&source, &delta)
}

#[test]
fn add_requirement() {
    let result = merge_fixture("add_requirement").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn remove_requirement() {
    let result = merge_fixture("remove_requirement").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn replace_requirement() {
    let result = merge_fixture("replace_requirement").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn rename_requirement() {
    let result = merge_fixture("rename_requirement").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn anchor_add_scenario() {
    let result = merge_fixture("anchor_add_scenario").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn mixed_operations() {
    let result = merge_fixture("mixed_operations").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn replace_summary() {
    let result = merge_fixture("replace_summary").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn delete_document() {
    let result = merge_fixture("delete").expect("merge should succeed");
    assert!(result.is_none(), "should signal deletion");
}

#[test]
fn anchor_replace_body() {
    let result = merge_fixture("anchor_replace_body").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn rename_then_modify() {
    let result = merge_fixture("rename_then_modify").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn doc_add_section() {
    let result = merge_fixture("doc_add_section").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn doc_replace_section() {
    let result = merge_fixture("doc_replace_section").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

// ---------------------------------------------------------------------------
// Validated merge wrappers
// ---------------------------------------------------------------------------

const SPEC_SOURCE: &str = "\
# Foo

A capability.

## Requirement: Bar

The system SHALL bar.

> test: code

### Scenario: Baz

- **WHEN** x happens
- **THEN** y follows
";

const DOC_SOURCE: &str = "\
# Foo

A capability doc.

## Overview

Some prose about Foo.
";

/// @spec merge/validate Validated merge outcome: A successful spec merge returns the rendered markdown and the parsed spec
#[test]
fn spec_merge_returns_rendered_and_parsed_spec() {
    let delta = "\
# @ Foo

## @ Requirement: Bar

### + Scenario: Qux

- **WHEN** something happens
- **THEN** a result follows

> test: code
";
    let merged = merge_spec_delta(SPEC_SOURCE, delta).expect("merge should succeed");
    match merged {
        Merged::Updated { rendered, artifact } => {
            assert!(
                rendered.contains("Scenario: Qux"),
                "rendered markdown carries the new scenario:\n{rendered}"
            );
            // The re-parsed spec carries both scenarios under requirement Bar.
            let bar = artifact
                .requirements
                .iter()
                .find(|r| r.name == "Bar")
                .expect("requirement Bar");
            let names: Vec<&str> = bar.scenarios.iter().map(|s| s.name.as_str()).collect();
            assert!(
                names.contains(&"Baz") && names.contains(&"Qux"),
                "got {names:?}"
            );
        }
        Merged::Deleted => panic!("expected an update, got a deletion"),
    }
}

/// @spec merge/validate Validated merge outcome: A delta that deletes the artifact yields a deletion outcome
#[test]
fn spec_merge_deletion_yields_deleted_outcome() {
    let delta = "# - Foo\n";
    let merged = merge_spec_delta(SPEC_SOURCE, delta).expect("merge should succeed");
    assert!(
        matches!(merged, Merged::Deleted),
        "a remove marker on the H1 yields a deletion outcome carrying no rendered text"
    );
}

/// @spec merge/validate Validated merge outcome: A doc merge is validated with the document parser
#[test]
fn doc_merge_returns_rendered_and_parsed_document() {
    let delta = "\
# @ Foo

## + Details

Extra prose about Foo.
";
    let merged = merge_doc_delta(DOC_SOURCE, delta).expect("merge should succeed");
    match merged {
        Merged::Updated { rendered, artifact } => {
            assert!(
                rendered.contains("Details"),
                "rendered markdown carries the new section:\n{rendered}"
            );
            assert_eq!(artifact.title, "Foo", "parsed document carries the title");
        }
        Merged::Deleted => panic!("expected an update, got a deletion"),
    }
}

/// @spec merge/validate Failure classification: A delta that does not apply returns a merge error
#[test]
fn delta_targeting_absent_heading_returns_merge_error() {
    let delta = "\
# @ Foo

## @ Requirement: Nonexistent

### + Scenario: New

- **WHEN** a
- **THEN** b

> test: code
";
    let err = merge_spec_delta(SPEC_SOURCE, delta).expect_err("merge should fail");
    assert!(
        matches!(err, MergeValidateError::Merge(_)),
        "a delta targeting an absent heading is a merge error, got: {err:?}"
    );
}

/// @spec merge/validate Failure classification: Merged text that violates its schema returns a parse error
#[test]
fn cleanly_applied_but_invalid_result_returns_parse_error() {
    // Adds a scenario with no WHEN/THEN — applies cleanly to the heading tree
    // but the merged spec no longer satisfies the schema.
    let delta = "\
# @ Foo

## @ Requirement: Bar

### + Scenario: Broken

- **GIVEN** only a given

> test: code
";
    let err = merge_spec_delta(SPEC_SOURCE, delta).expect_err("merge should fail");
    assert!(
        matches!(err, MergeValidateError::Parse(_)),
        "merged text that violates the schema is a parse error, got: {err:?}"
    );
}

/// @spec merge/validate Failure classification: A multi-error failure renders as one summarized line
#[test]
fn multi_error_failure_renders_as_one_summarized_line() {
    let rendered = summarize_errors(&["first problem", "second problem", "third problem"]);
    assert_eq!(rendered, "first problem (and 2 more)");

    // A single error renders as just its message.
    assert_eq!(summarize_errors(&["only problem"]), "only problem");
}
