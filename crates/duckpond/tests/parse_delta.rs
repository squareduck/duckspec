mod common;

use duckpond::parse;
use duckpond::parse::delta::parse_delta;

fn parse_fixture(
    name: &str,
) -> Result<duckpond::artifact::delta::Delta, Vec<duckpond::error::ParseError>> {
    let source = common::load_fixture("delta", name);
    let elements = parse::parse_elements(&source);
    parse_delta(&elements)
}

// @spec parse/delta Delta document structure: Delta with a single Add entry parses successfully
#[test]
fn add_only() {
    let delta = parse_fixture("add_only.md").expect("should parse");
    insta::assert_debug_snapshot!(delta);
}

// @spec parse/delta Delta document structure: Delta with mixed-marker entries is sorted into canonical order
#[test]
fn mixed_ops_sorted_canonically() {
    // The fixture has entries in non-canonical order (@ + = -).
    // The parser should sort them to canonical order (= - @ +).
    let delta = parse_fixture("mixed_ops.md").expect("should parse");
    insta::assert_debug_snapshot!(delta);
}

// @spec parse/delta Delta document structure: Delta with rename and anchor entries parses with new-name extraction
#[test]
fn rename_with_anchor() {
    let delta = parse_fixture("rename_with_anchor.md").expect("should parse");
    insta::assert_debug_snapshot!(delta);
}

// @spec parse/delta Marker rules: Marker without a following space raises MarkerMissingSpace
#[test]
fn err_marker_missing_space() {
    let errors = parse_fixture("errors/marker_missing_space.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/delta Marker rules: Add marker on H1 raises AddOnH1
#[test]
fn err_add_on_h1() {
    let errors = parse_fixture("errors/add_on_h1.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/delta Marker rules: H2 without a marker raises MissingDeltaMarker
#[test]
fn err_missing_marker_on_h2() {
    let errors = parse_fixture("errors/missing_marker_on_h2.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/delta Marker rules: Anchor marker on H3 raises AnchorOnH3
#[test]
fn err_anchor_on_h3() {
    let errors = parse_fixture("errors/anchor_on_h3.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/delta Marker rules: Marker on a content child raises MarkerOnContentChild
#[test]
fn err_marker_on_content_child() {
    let errors = parse_fixture("errors/marker_on_content_child.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/delta Per-marker entry semantics: Remove with body or rename without new-name line raises the relevant invariant
#[test]
fn err_remove_and_rename_invariants() {
    let errors = parse_fixture("errors/remove_and_rename_invariants.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}
