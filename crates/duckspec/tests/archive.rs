//! End-to-end tests for the `ds archive` orphan guard, driving the real `ds`
//! binary so exit status, filesystem effects, and stderr are all exercised.

use std::fs;
use std::path::Path;
use std::process::Command;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

/// Cap spec that defines scenario `Baz` (the one a source backlink points to).
const SPEC_WITH_BAZ: &str = "\
# Foo

A capability.

## Requirement: Bar

The system SHALL bar.

> test: code

### Scenario: Baz

- **WHEN** x happens
- **THEN** y follows
";

/// The change's replacement spec — drops `Baz`, so archiving it orphans the
/// live backlink.
const SPEC_WITHOUT_BAZ: &str = "\
# Foo

A capability.

## Requirement: Bar

The system SHALL bar.

> test: code

### Scenario: Qux

- **WHEN** x happens
- **THEN** y follows
";

/// Build a project whose archive of change `orphaner` removes the backlinked
/// scenario `Baz`. Returns the project root.
fn project_with_orphaning_change() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    let duckspec = root.join("duckspec");

    write(&duckspec.join("caps/foo/spec.md"), SPEC_WITH_BAZ);
    write(
        &duckspec.join("changes/orphaner/caps/foo/spec.md"),
        SPEC_WITHOUT_BAZ,
    );
    write(
        &root.join("tests/foo_test.rs"),
        "// @spec foo Bar: Baz\nfn t() {}\n",
    );

    project
}

fn ds(project_root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ds"))
        .args(args)
        .current_dir(project_root)
        .output()
        .expect("run ds")
}

/// @spec archive/backlink-guard Refusal and override: Refusal leaves the capabilities and change untouched
#[test]
fn refusal_leaves_capabilities_and_change_untouched() {
    let project = project_with_orphaning_change();
    let root = project.path();

    let output = ds(root, &["archive", "orphaner"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "archive must fail when it would orphan a backlink; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("foo_test.rs"),
        "failure must name the offending source file; stderr:\n{stderr}"
    );

    // Capability spec is untouched — still defines Baz.
    let spec = fs::read_to_string(root.join("duckspec/caps/foo/spec.md")).unwrap();
    assert_eq!(spec, SPEC_WITH_BAZ, "caps/foo/spec.md must be unchanged");

    // The change still lives under changes/, not archive/.
    assert!(
        root.join("duckspec/changes/orphaner").is_dir(),
        "change must remain under changes/"
    );
    assert!(
        !root.join("duckspec/archive").exists(),
        "nothing should have been moved to archive/"
    );
}

/// @spec archive/backlink-guard Refusal and override: allow-orphans completes the archive with a warning
#[test]
fn allow_orphans_completes_archive_with_warning() {
    let project = project_with_orphaning_change();
    let root = project.path();

    let output = ds(root, &["archive", "orphaner", "--allow-orphans"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "archive must complete with --allow-orphans; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("foo_test.rs"),
        "a warning must name the offending source file; stderr:\n{stderr}"
    );

    // The archive landed — caps/foo/spec.md now holds the replacement.
    let spec = fs::read_to_string(root.join("duckspec/caps/foo/spec.md")).unwrap();
    assert_eq!(spec, SPEC_WITHOUT_BAZ, "caps/foo/spec.md must be rewritten");

    // The change moved out of changes/ into archive/.
    assert!(
        !root.join("duckspec/changes/orphaner").is_dir(),
        "change must be moved out of changes/"
    );
    assert!(
        root.join("duckspec/archive").is_dir(),
        "change must be moved into archive/"
    );
}
