mod common;

use duckpond::parse;
use duckpond::parse::spec::parse_spec;

fn roundtrip(name: &str) -> String {
    let source = common::load_fixture("spec", name);
    let elements = parse::parse_elements(&source);
    let spec = parse_spec(&elements).expect("should parse");
    spec.render()
}

#[test]
fn render_minimal() {
    insta::assert_snapshot!(roundtrip("minimal.md"));
}

#[test]
fn render_multi_requirement() {
    insta::assert_snapshot!(roundtrip("multi_requirement.md"));
}

#[test]
fn render_test_inheritance() {
    insta::assert_snapshot!(roundtrip("test_inheritance.md"));
}

#[test]
fn render_with_description() {
    insta::assert_snapshot!(roundtrip("with_description.md"));
}

#[test]
fn render_with_backlinks() {
    insta::assert_snapshot!(roundtrip("with_backlinks.md"));
}
