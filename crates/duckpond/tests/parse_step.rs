mod common;

use duckpond::parse;
use duckpond::parse::step::parse_step;

fn parse_fixture(
    name: &str,
) -> Result<duckpond::artifact::step::Step, Vec<duckpond::error::ParseError>> {
    let source = common::load_fixture("step", name);
    let elements = parse::parse_elements(&source);
    parse_step(&elements)
}

#[test]
fn minimal_step() {
    let step = parse_fixture("minimal.md").expect("should parse");
    insta::assert_debug_snapshot!(step);
}

#[test]
fn with_prerequisites() {
    let step = parse_fixture("with_prerequisites.md").expect("should parse");
    insta::assert_debug_snapshot!(step);
}

#[test]
fn with_context() {
    let step = parse_fixture("with_context.md").expect("should parse");
    insta::assert_debug_snapshot!(step);
}
