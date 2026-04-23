use std::fs;
use std::path::{Path, PathBuf};

use duckpond::audit::{self, AuditScope};
use duckpond::config::Config;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

const CHANGE_CAP_SPEC: &str = "\
# Bar

A new in-flight capability.

## Requirement: Bar behavior

The system SHALL do the bar thing.

> test: code

### Scenario: Happy path

- **WHEN** the user does bar
- **THEN** the system confirms

> test: code
> - tests/bar_test.rs:1
";

const STEP: &str = "\
# Implement bar

Wire up the bar behavior.

## Tasks

- [ ] Write the implementation
- [ ] @spec foo/bar Bar behavior: Happy path
";

const BACKLINK_SOURCE: &str = "\
// @spec foo/bar Bar behavior: Happy path
fn test_bar_happy_path() {
    assert_eq!(1, 1);
}
";

/// When a step's backlink points to a scenario that is defined in an
/// active (not-yet-archived) change, audit should treat it as resolved.
/// Before the fix, the backlink was only resolved against main `caps/`,
/// so every intermediate state of a multi-step change reported spurious
/// `unresolved_backlinks` errors.
#[test]
fn backlink_to_in_flight_change_scenario_resolves() {
    let project = tempfile::tempdir().unwrap();
    let duckspec = project.path().join("duckspec");

    write(
        &duckspec.join("changes/add-bar/caps/foo/bar/spec.md"),
        CHANGE_CAP_SPEC,
    );
    write(&duckspec.join("changes/add-bar/steps/01-impl.md"), STEP);
    write(&project.path().join("tests/bar_test.rs"), BACKLINK_SOURCE);

    let config = Config::load(&duckspec).unwrap();
    let report = audit::run_audit(&duckspec, project.path(), &config, AuditScope::Full)
        .expect("audit runs");

    assert!(
        report.unresolved_backlinks.is_empty(),
        "expected no unresolved backlinks, got: {:?}",
        report
            .unresolved_backlinks
            .iter()
            .map(|b| b.key.display())
            .collect::<Vec<_>>()
    );
    assert!(
        report.unresolved_step_refs.is_empty(),
        "expected no unresolved step refs, got: {:?}",
        report
            .unresolved_step_refs
            .iter()
            .map(|r| r.key.display())
            .collect::<Vec<_>>()
    );
}

/// An unresolved step `@spec` task ref must carry the step file path and
/// line number so consumers (CLI output, GUI diagnostics) can attribute
/// the error to a specific step file.
#[test]
fn unresolved_step_ref_records_step_file_and_line() {
    let project = tempfile::tempdir().unwrap();
    let duckspec = project.path().join("duckspec");

    // Step references a scenario that exists nowhere — not in main caps
    // and not in any active change.
    let step_body = "\
# Implement missing

Wire up the missing behavior.

## Tasks

- [ ] Some setup task
- [ ] @spec ghost Nothing: Nowhere
";
    write(
        &duckspec.join("changes/add-ghost/steps/01-impl.md"),
        step_body,
    );

    let config = Config::load(&duckspec).unwrap();
    let report = audit::run_audit(&duckspec, project.path(), &config, AuditScope::Full)
        .expect("audit runs");

    assert_eq!(
        report.unresolved_step_refs.len(),
        1,
        "expected exactly one unresolved step ref"
    );
    let r = &report.unresolved_step_refs[0];
    assert_eq!(r.change_name, "add-ghost");
    assert_eq!(
        r.step_file,
        PathBuf::from("changes/add-ghost/steps/01-impl.md"),
    );
    // `@spec` task is on line 8 of the step body above.
    assert_eq!(r.line, 8);
}

/// A backlink that points to a scenario that does not exist anywhere —
/// not in main caps, not in any active change — must still fail.
#[test]
fn backlink_to_unknown_scenario_still_fails() {
    let project = tempfile::tempdir().unwrap();
    let duckspec = project.path().join("duckspec");

    fs::create_dir_all(duckspec.join("caps")).unwrap();
    write(
        &project.path().join("tests/missing_test.rs"),
        "// @spec ghost Nothing: Nowhere\nfn t() {}\n",
    );

    let config = Config::load(&duckspec).unwrap();
    let report = audit::run_audit(&duckspec, project.path(), &config, AuditScope::Full)
        .expect("audit runs");

    assert_eq!(
        report.unresolved_backlinks.len(),
        1,
        "expected exactly one unresolved backlink for an unknown scenario"
    );
}
