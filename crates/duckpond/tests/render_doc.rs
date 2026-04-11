mod common;

use duckpond::parse;
use duckpond::parse::doc::parse_document;

fn roundtrip(name: &str) -> String {
    let source = common::load_fixture("doc", name);
    let elements = parse::parse_elements(&source);
    let doc = parse_document(&elements).expect("should parse");
    doc.render()
}

#[test]
fn render_minimal() {
    insta::assert_snapshot!(roundtrip("minimal.md"));
}

#[test]
fn render_with_sections() {
    insta::assert_snapshot!(roundtrip("with_sections.md"));
}

#[test]
fn render_nested_headings() {
    insta::assert_snapshot!(roundtrip("nested_headings.md"));
}
