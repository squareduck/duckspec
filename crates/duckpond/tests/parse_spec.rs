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

#[test]
fn minimal_spec() {
    let spec = parse_fixture("minimal.md").expect("should parse");
    insta::assert_debug_snapshot!(spec);
}

#[test]
fn multi_requirement() {
    let spec = parse_fixture("multi_requirement.md").expect("should parse");
    insta::assert_debug_snapshot!(spec);
}

#[test]
fn test_inheritance() {
    let spec = parse_fixture("test_inheritance.md").expect("should parse");
    insta::assert_debug_snapshot!(spec);
}

#[test]
fn with_description() {
    let spec = parse_fixture("with_description.md").expect("should parse");
    insta::assert_debug_snapshot!(spec);
}

#[test]
fn with_backlinks() {
    let spec = parse_fixture("with_backlinks.md").expect("should parse");
    insta::assert_debug_snapshot!(spec);
}
