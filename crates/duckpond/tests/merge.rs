mod common;

use duckpond::merge::apply_delta;

fn merge_fixture(name: &str) -> Result<Option<String>, Vec<duckpond::error::MergeError>> {
    let source = common::load_fixture("merge", &format!("{name}_source.md"));
    let delta = common::load_fixture("merge", &format!("{name}_delta.md"));
    apply_delta(&source, &delta)
}

#[test]
fn add_requirement() {
    let result = merge_fixture("add_requirement").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn remove_requirement() {
    let result = merge_fixture("remove_requirement").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn replace_requirement() {
    let result = merge_fixture("replace_requirement").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn rename_requirement() {
    let result = merge_fixture("rename_requirement").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn anchor_add_scenario() {
    let result = merge_fixture("anchor_add_scenario").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn mixed_operations() {
    let result = merge_fixture("mixed_operations").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn replace_summary() {
    let result = merge_fixture("replace_summary").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn delete_document() {
    let result = merge_fixture("delete").expect("merge should succeed");
    assert!(result.is_none(), "should signal deletion");
}

#[test]
fn anchor_replace_body() {
    let result = merge_fixture("anchor_replace_body").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn rename_then_modify() {
    let result = merge_fixture("rename_then_modify").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn doc_add_section() {
    let result = merge_fixture("doc_add_section").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}

#[test]
fn doc_replace_section() {
    let result = merge_fixture("doc_replace_section").expect("merge should succeed");
    let output = result.expect("should not be deleted");
    insta::assert_snapshot!(output);
}
