mod common;

use duckpond::parse;
use duckpond::parse::delta::parse_delta;

fn roundtrip(name: &str) -> String {
    let source = common::load_fixture("delta", name);
    let elements = parse::parse_elements(&source);
    let delta = parse_delta(&elements).expect("should parse");
    delta.render()
}

#[test]
fn render_add_only() {
    insta::assert_snapshot!(roundtrip("add_only.md"));
}

#[test]
fn render_mixed_ops() {
    // Input has non-canonical order; rendered output should be canonical.
    insta::assert_snapshot!(roundtrip("mixed_ops.md"));
}

#[test]
fn render_rename_with_anchor() {
    insta::assert_snapshot!(roundtrip("rename_with_anchor.md"));
}
