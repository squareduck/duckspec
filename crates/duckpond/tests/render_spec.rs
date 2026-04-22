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

#[test]
fn render_preserves_wrapped_gwt_continuations() {
    let source = "\
# Session expiration

Sessions expire.

## Requirement: Idle timeout

The system SHALL expire sessions.

### Scenario: Idle user

- **GIVEN** an authenticated user with a long description
  that wraps onto a continuation line
- **WHEN** the user makes no requests for 30 minutes and
  the clock advances past the threshold
- **THEN** the next request returns 401

> test: code
";
    let elements = parse::parse_elements(source);
    let spec = parse_spec(&elements).expect("should parse");
    let rendered = spec.render();
    assert_eq!(
        rendered, source,
        "wrapped GWT continuation lines must roundtrip with 2-space indentation preserved"
    );
}
