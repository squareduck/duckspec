mod common;

use duckpond::parse;
use duckpond::parse::doc::parse_document;

fn parse_fixture(
    name: &str,
) -> Result<duckpond::artifact::doc::Document, Vec<duckpond::error::ParseError>> {
    let source = common::load_fixture("doc", name);
    let elements = parse::parse_elements(&source);
    parse_document(&elements)
}

#[test]
fn minimal_doc() {
    let doc = parse_fixture("minimal.md").expect("should parse");
    insta::assert_debug_snapshot!(doc);
}

#[test]
fn with_sections() {
    let doc = parse_fixture("with_sections.md").expect("should parse");
    insta::assert_debug_snapshot!(doc);
}

#[test]
fn nested_headings() {
    let doc = parse_fixture("nested_headings.md").expect("should parse");
    insta::assert_debug_snapshot!(doc);
}
