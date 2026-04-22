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
    // GWT clauses long enough to wrap at the default 90-char width even after
    // accounting for the `- **GIVEN** ` prefix.
    let source = "\
# Session expiration

Sessions expire.

## Requirement: Idle timeout

The system SHALL expire sessions.

### Scenario: Idle user

- **GIVEN** an authenticated user whose session description carries quite a lot of
  contextual information about their identity and how they originally signed in

- **WHEN** the user makes absolutely no requests for thirty minutes and the wall clock
  advances well past the configured idle threshold

- **THEN** the next request returns a 401 response with a body explaining that the session
  has expired and a fresh login is required

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
