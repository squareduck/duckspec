mod common;

use duckpond::parse;
use duckpond::parse::step::parse_step;

fn roundtrip(name: &str) -> String {
    let source = common::load_fixture("step", name);
    let elements = parse::parse_elements(&source);
    let step = parse_step(&elements).expect("should parse");
    step.render()
}

#[test]
fn render_minimal() {
    insta::assert_snapshot!(roundtrip("minimal.md"));
}

#[test]
fn render_with_prerequisites() {
    insta::assert_snapshot!(roundtrip("with_prerequisites.md"));
}

#[test]
fn render_with_context() {
    insta::assert_snapshot!(roundtrip("with_context.md"));
}
