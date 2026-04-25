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

// @spec parse/doc Document structure: Minimal document parses successfully
#[test]
fn minimal_doc() {
    let doc = parse_fixture("minimal.md").expect("should parse");
    insta::assert_debug_snapshot!(doc);
}

// @spec parse/doc Section tree: Document with sibling H2 sections parses into a flat section list
#[test]
fn with_sections() {
    let doc = parse_fixture("with_sections.md").expect("should parse");
    insta::assert_debug_snapshot!(doc);
}

// @spec parse/doc Section tree: Document with nested headings produces a parent-child section tree
#[test]
fn nested_headings() {
    let doc = parse_fixture("nested_headings.md").expect("should parse");
    insta::assert_debug_snapshot!(doc);
}

// @spec parse/doc Section tree: Section tree captures headings nested four levels deep
#[test]
fn deep_recursion() {
    let doc = parse_fixture("deep_recursion.md").expect("should parse");
    insta::assert_debug_snapshot!(doc);
}
