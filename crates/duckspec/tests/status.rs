//! End-to-end tests for `ds status <change>` progress coverage.

use std::fs;
use std::path::Path;
use std::process::Command;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

const CHANGE_CAP: &str = "\
# Foo

A new capability.

## Requirement: Behavior

The system SHALL behave.

> test: code

### Scenario: Alpha

- **WHEN** x happens
- **THEN** y follows

> test: code
> - tests/phantom.rs:1
";

fn ds(project_root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ds"))
        .args(args)
        .current_dir(project_root)
        .output()
        .expect("run ds")
}

/// @spec status/change-coverage Change status surfaces the partition: Open scenario appears in change status open list
#[test]
fn open_scenario_appears_in_change_status_open_list() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    let duckspec = root.join("duckspec");

    write(
        &duckspec.join("changes/add-foo/caps/foo/spec.md"),
        CHANGE_CAP,
    );

    let output = ds(root, &["status", "add-foo"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "status must succeed; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("open:"),
        "open progress heading expected; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("@spec foo Behavior: Alpha"),
        "open scenario must be listed; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("missing:"),
        "progress language is open, not missing; stderr:\n{stderr}"
    );
}

/// @spec status/change-coverage Change status surfaces the partition: Linked scenario does not appear as missing or open
#[test]
fn linked_scenario_does_not_appear_as_missing_or_open() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    let duckspec = root.join("duckspec");

    write(
        &duckspec.join("changes/add-foo/caps/foo/spec.md"),
        CHANGE_CAP,
    );
    // Source backlink resolves; marker path list is irrelevant.
    write(
        &root.join("tests/foo_alpha.rs"),
        "// @spec foo Behavior: Alpha\nfn t() {}\n",
    );

    let output = ds(root, &["status", "add-foo"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "status must succeed; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("1/1 scenarios linked") || stderr.contains("scenarios linked"),
        "linked progress expected; stderr:\n{stderr}"
    );
    // Linked scenario must not appear under open or missing.
    let has_open_section = stderr.lines().any(|l| l.trim() == "open:");
    let has_missing_section = stderr.lines().any(|l| l.trim() == "missing:");
    assert!(
        !has_open_section,
        "linked-only change must not list open; stderr:\n{stderr}"
    );
    assert!(
        !has_missing_section,
        "linked scenario must not be listed as missing; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("@spec foo Behavior: Alpha"),
        "linked scenario must not appear as a problem line; stderr:\n{stderr}"
    );
}
