mod common;

use duckpond::parse;
use duckpond::parse::spec::parse_spec;

fn parse_fixture(
    name: &str,
) -> Result<duckpond::artifact::spec::Spec, Vec<duckpond::error::ParseError>> {
    let source = common::load_fixture("spec", name);
    let elements = parse::parse_elements(&source);
    parse_spec(&elements)
}

// @spec parse/spec Spec document structure: Minimal spec parses successfully
#[test]
fn minimal_spec() {
    let spec = parse_fixture("minimal.md").expect("should parse");
    insta::assert_debug_snapshot!(spec);
}

// @spec parse/spec Requirement section structure: Spec with multiple requirements parses successfully
#[test]
fn multi_requirement() {
    let spec = parse_fixture("multi_requirement.md").expect("should parse");
    insta::assert_debug_snapshot!(spec);
}

// @spec parse/spec Test markers: Test marker inherits from requirement to scenario
#[test]
fn test_inheritance() {
    let spec = parse_fixture("test_inheritance.md").expect("should parse");
    insta::assert_debug_snapshot!(spec);
}

// @spec parse/spec Spec document structure: Spec with multi-paragraph description parses successfully
#[test]
fn with_description() {
    let spec = parse_fixture("with_description.md").expect("should parse");
    insta::assert_debug_snapshot!(spec);
}

// @spec parse/spec Test markers: Test code marker carries backlinks
#[test]
fn with_backlinks() {
    let spec = parse_fixture("with_backlinks.md").expect("should parse");
    insta::assert_debug_snapshot!(spec);
}

// -- Error fixtures ----------------------------------------------------------

// @spec parse/spec Spec document structure: Content before H1 raises ContentBeforeH1
#[test]
fn err_content_before_h1() {
    let errors = parse_fixture("errors/content_before_h1.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/spec Spec document structure: Missing H1 raises MissingH1
#[test]
fn err_missing_h1() {
    let errors = parse_fixture("errors/missing_h1.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/spec Spec document structure: Missing summary raises MissingSummary
#[test]
fn err_missing_summary() {
    let errors = parse_fixture("errors/missing_summary.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/spec Spec document structure: Headings deeper than H3 raise HeadingTooDeep
#[test]
fn err_heading_too_deep() {
    let errors = parse_fixture("errors/heading_too_deep.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/spec Requirement section structure: H2 without 'Requirement: ' prefix raises InvalidRequirementPrefix
#[test]
fn err_invalid_requirement_prefix() {
    let errors = parse_fixture("errors/invalid_requirement_prefix.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/spec Requirement section structure: Requirement name containing a colon raises RequirementNameColon
#[test]
fn err_requirement_name_colon() {
    let errors = parse_fixture("errors/requirement_name_colon.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/spec Requirement section structure: Requirement with neither prose nor scenarios raises EmptyRequirement
#[test]
fn err_empty_requirement() {
    let errors = parse_fixture("errors/empty_requirement.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/spec Scenario section structure: H3 without 'Scenario: ' prefix raises InvalidScenarioPrefix
#[test]
fn err_invalid_scenario_prefix() {
    let errors = parse_fixture("errors/invalid_scenario_prefix.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/spec Scenario section structure: Scenario missing WHEN or THEN raises MissingWhen and MissingThen
#[test]
fn err_missing_when_or_then() {
    let errors = parse_fixture("errors/missing_when_or_then.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/spec Scenario section structure: Non-GWT content inside scenario body raises UnexpectedScenarioContent
#[test]
fn err_unexpected_scenario_content() {
    let errors = parse_fixture("errors/unexpected_scenario_content.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/spec GWT phase machine: Out-of-order GWT clauses raise GwtClauseOutOfOrder
#[test]
fn err_gwt_out_of_order() {
    let errors = parse_fixture("errors/gwt_out_of_order.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/spec GWT phase machine: Unrecognized GWT keyword raises InvalidGwtKeyword
#[test]
fn err_invalid_gwt_keyword() {
    let errors = parse_fixture("errors/invalid_gwt_keyword.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/spec Test markers: Unrecognized test marker prefix raises InvalidTestMarker
#[test]
fn err_invalid_test_marker() {
    let errors = parse_fixture("errors/invalid_test_marker.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/spec Test markers: Scenario with no marker and requirement with no marker raises UnresolvedTestMarker
#[test]
fn err_unresolved_test_marker() {
    let errors = parse_fixture("errors/unresolved_test_marker.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}
