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

// @spec parse/step Step document structure: Minimal step parses successfully
#[test]
fn minimal_step() {
    let step = parse_fixture("minimal.md").expect("should parse");
    insta::assert_debug_snapshot!(step);
}

// @spec parse/step Step document structure: Step with prerequisites parses successfully
#[test]
fn with_prerequisites() {
    let step = parse_fixture("with_prerequisites.md").expect("should parse");
    insta::assert_debug_snapshot!(step);
}

// @spec parse/step Step document structure: Step with context parses successfully
#[test]
fn with_context() {
    let step = parse_fixture("with_context.md").expect("should parse");
    insta::assert_debug_snapshot!(step);
}

// @spec parse/step Tasks section: Tasks containing @spec references parse as SpecRef content
#[test]
fn with_spec_refs() {
    let step = parse_fixture("with_spec_refs.md").expect("should parse");
    insta::assert_debug_snapshot!(step);
}

// @spec parse/step Prerequisites section: Prerequisites with @step references parse as StepRef kind
#[test]
fn with_step_refs() {
    let step = parse_fixture("with_step_refs.md").expect("should parse");
    insta::assert_debug_snapshot!(step);
}

// @spec parse/step Tasks section: Task checkboxes and numeric prefixes are recognized and stripped
#[test]
fn with_checkboxes_and_prefixes() {
    let step = parse_fixture("with_checkboxes_and_prefixes.md").expect("should parse");
    insta::assert_debug_snapshot!(step);
}

// @spec parse/step Step document structure: Unknown section heading raises UnknownStepSection
#[test]
fn err_unknown_section() {
    let errors = parse_fixture("errors/unknown_section.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}

// @spec parse/step Tasks section: Missing or empty Tasks section raises the relevant error
#[test]
fn err_tasks_missing_or_empty() {
    let missing = parse_fixture("errors/tasks_missing.md").expect_err("should fail");
    let empty = parse_fixture("errors/tasks_empty.md").expect_err("should fail");
    insta::assert_debug_snapshot!((missing, empty));
}

// @spec parse/step Tasks section: Subtask indented beyond four spaces raises SubtaskTooDeep
#[test]
fn err_subtask_too_deep() {
    let errors = parse_fixture("errors/subtask_too_deep.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}
