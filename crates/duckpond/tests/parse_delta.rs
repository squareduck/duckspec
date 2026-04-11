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

#[test]
fn add_only() {
    let delta = parse_fixture("add_only.md").expect("should parse");
    insta::assert_debug_snapshot!(delta);
}

#[test]
fn mixed_ops_sorted_canonically() {
    // The fixture has entries in non-canonical order (@ + = -).
    // The parser should sort them to canonical order (= - @ +).
    let delta = parse_fixture("mixed_ops.md").expect("should parse");
    insta::assert_debug_snapshot!(delta);
}

#[test]
fn rename_with_anchor() {
    let delta = parse_fixture("rename_with_anchor.md").expect("should parse");
    insta::assert_debug_snapshot!(delta);
}
