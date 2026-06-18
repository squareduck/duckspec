use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use duckpond::audit::{self, AuditScope, ProjectedSpec};
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

// Written inline with `\n` escapes (rather than a `"\` block) so the marker is
// not on its own physical line in this source file — otherwise the project's
// own backlink scan would treat this fixture as a real, unresolved backlink.
const BACKLINK_SOURCE: &str =
    "// @spec foo/bar Bar behavior: Happy path\nfn test_bar_happy_path() {\n    assert_eq!(1, 1);\n}\n";

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

// ---------------------------------------------------------------------------
// Archive orphan guard — would_be_orphaned
// ---------------------------------------------------------------------------

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

const SPEC_WITH_BAZ_AND_MORE: &str = "\
# Foo

A capability.

## Requirement: Bar

The system SHALL bar.

> test: code

### Scenario: Baz

- **WHEN** x happens
- **THEN** y follows

### Scenario: Qux

- **WHEN** x happens
- **THEN** y follows
";

/// @spec archive/backlink-guard Orphan detection: Archiving a change that removes a backlinked scenario flags the backlink
#[test]
fn archiving_removed_scenario_flags_backlink() {
    let project = tempfile::tempdir().unwrap();
    let duckspec = project.path().join("duckspec");

    write(&duckspec.join("caps/foo/spec.md"), SPEC_WITH_BAZ);
    write(
        &project.path().join("tests/foo_test.rs"),
        "// @spec foo Bar: Baz\nfn t() {}\n",
    );

    let config = Config::load(&duckspec).unwrap();
    let mut projected = HashMap::new();
    projected.insert(
        "foo".to_string(),
        ProjectedSpec::Updated(SPEC_WITHOUT_BAZ.to_string()),
    );

    let orphans =
        audit::would_be_orphaned(project.path(), &duckspec, &config, &projected).expect("guard");

    assert_eq!(orphans.len(), 1, "the removed scenario's backlink is flagged");
    assert!(orphans[0].source_file.ends_with("tests/foo_test.rs"));
    assert_eq!(orphans[0].key.scenario, "Baz");
}

/// @spec archive/backlink-guard Orphan detection: An archive that preserves every backlinked scenario reports no orphans
#[test]
fn preserving_backlinked_scenario_reports_no_orphans() {
    let project = tempfile::tempdir().unwrap();
    let duckspec = project.path().join("duckspec");

    write(&duckspec.join("caps/foo/spec.md"), SPEC_WITH_BAZ);
    write(
        &project.path().join("tests/foo_test.rs"),
        "// @spec foo Bar: Baz\nfn t() {}\n",
    );

    let config = Config::load(&duckspec).unwrap();
    // Archive adds a scenario but keeps the backlinked one (Baz).
    let mut projected = HashMap::new();
    projected.insert(
        "foo".to_string(),
        ProjectedSpec::Updated(SPEC_WITH_BAZ_AND_MORE.to_string()),
    );

    let orphans =
        audit::would_be_orphaned(project.path(), &duckspec, &config, &projected).expect("guard");

    assert!(
        orphans.is_empty(),
        "no orphans when every backlinked scenario survives, got: {:?}",
        orphans.iter().map(|o| o.key.display()).collect::<Vec<_>>()
    );
}

/// @spec archive/backlink-guard Orphan detection: A backlink already unresolved before the archive is not attributed to it
#[test]
fn preexisting_unresolved_backlink_not_attributed() {
    let project = tempfile::tempdir().unwrap();
    let duckspec = project.path().join("duckspec");

    write(&duckspec.join("caps/foo/spec.md"), SPEC_WITH_BAZ);
    // Backlink points at a scenario that does not exist today.
    write(
        &project.path().join("tests/foo_test.rs"),
        "// @spec foo Bar: Ghost\nfn t() {}\n",
    );

    let config = Config::load(&duckspec).unwrap();
    // The archive does not introduce Ghost either.
    let mut projected = HashMap::new();
    projected.insert(
        "foo".to_string(),
        ProjectedSpec::Updated(SPEC_WITHOUT_BAZ.to_string()),
    );

    let orphans =
        audit::would_be_orphaned(project.path(), &duckspec, &config, &projected).expect("guard");

    assert!(
        orphans.is_empty(),
        "a backlink already unresolved before the archive is not an orphan, got: {:?}",
        orphans.iter().map(|o| o.key.display()).collect::<Vec<_>>()
    );
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

// ---------------------------------------------------------------------------
// Change-scoped progress classification — pending vs error
// ---------------------------------------------------------------------------

/// Build a change cap spec for a new capability `foo` whose requirement
/// "Behavior" holds the named scenarios, all inheriting `test: code`.
fn change_spec(scenarios: &[&str]) -> String {
    let mut s = String::from(
        "# Foo\n\nA new capability.\n\n## Requirement: Behavior\n\nThe system SHALL behave.\n\n> test: code\n",
    );
    for name in scenarios {
        s.push_str(&format!(
            "\n### Scenario: {name}\n\n- **WHEN** x happens\n- **THEN** y follows\n"
        ));
    }
    s
}

/// Build a step body whose Tasks section holds one `@spec foo Behavior: <name>`
/// task per entry, checked according to the bool. The H1 slug ("implement")
/// matches the `01-implement.md` filename used by the tests below.
fn step_body(refs: &[(&str, bool)]) -> String {
    let mut s = String::from("# Implement\n\nDo the work.\n\n## Tasks\n");
    for (name, checked) in refs {
        let mark = if *checked { "x" } else { " " };
        s.push_str(&format!("- [{mark}] @spec foo Behavior: {name}\n"));
    }
    s
}

/// A source backlink for `foo Behavior: <scenario>`, written with `\n` escapes
/// so the marker never sits on its own physical line in this file.
fn backlink_source(scenario: &str) -> String {
    format!("// @spec foo Behavior: {scenario}\nfn t() {{}}\n")
}

/// Whether any key in the set names the given scenario.
fn contains(keys: &[audit::ScenarioKey], scenario: &str) -> bool {
    keys.iter().any(|k| k.scenario == scenario)
}

/// Set up a change `add-foo` with the given spec scenarios and step refs,
/// then run a change-scoped audit. `backlinked` lists scenarios that get a
/// real source backlink.
fn scoped_report(
    spec_scenarios: &[&str],
    step_refs: &[(&str, bool)],
    backlinked: &[&str],
) -> audit::AuditReport {
    let project = tempfile::tempdir().unwrap();
    let duckspec = project.path().join("duckspec");

    write(
        &duckspec.join("changes/add-foo/caps/foo/spec.md"),
        &change_spec(spec_scenarios),
    );
    write(
        &duckspec.join("changes/add-foo/steps/01-implement.md"),
        &step_body(step_refs),
    );
    for (i, scenario) in backlinked.iter().enumerate() {
        write(
            &project.path().join(format!("tests/foo_{i}.rs")),
            &backlink_source(scenario),
        );
    }

    let config = Config::load(&duckspec).unwrap();
    audit::run_audit(
        &duckspec,
        project.path(),
        &config,
        AuditScope::Change("add-foo".to_string()),
    )
    .expect("audit runs")
}

/// @spec audit/change-progress Classify unlinked scenarios by step completion: Unchecked referencing task is pending
#[test]
fn unchecked_referencing_task_is_pending() {
    let report = scoped_report(&["Alpha"], &[("Alpha", false)], &[]);

    assert!(
        contains(&report.pending_backlink_scenarios, "Alpha"),
        "an unlinked scenario whose only step task is unchecked is pending"
    );
    assert!(
        !contains(&report.missing_backlink_scenarios, "Alpha"),
        "pending scenarios are not errors"
    );
}

/// @spec audit/change-progress Classify unlinked scenarios by step completion: Checked referencing task is an error
#[test]
fn checked_referencing_task_is_an_error() {
    let report = scoped_report(&["Alpha"], &[("Alpha", true)], &[]);

    assert!(
        contains(&report.missing_backlink_scenarios, "Alpha"),
        "an unlinked scenario claimed by a checked task is an error"
    );
    assert!(
        !contains(&report.pending_backlink_scenarios, "Alpha"),
        "a claimed scenario is not pending"
    );
}

/// @spec audit/change-progress Classify unlinked scenarios by step completion: A scenario claimed by any checked task is an error
#[test]
fn scenario_claimed_by_any_checked_task_is_an_error() {
    // Two tasks reference Alpha — one checked, one not. Any checked claim wins.
    let report = scoped_report(&["Alpha"], &[("Alpha", false), ("Alpha", true)], &[]);

    assert!(
        contains(&report.missing_backlink_scenarios, "Alpha"),
        "any checked referencing task claims the scenario, making it an error"
    );
    assert!(!contains(&report.pending_backlink_scenarios, "Alpha"));
}

/// @spec audit/change-progress Classify unlinked scenarios by step completion: A backlinked scenario is neither pending nor an error
#[test]
fn backlinked_scenario_is_neither_pending_nor_an_error() {
    let report = scoped_report(&["Alpha"], &[("Alpha", true)], &["Alpha"]);

    assert!(!contains(&report.missing_backlink_scenarios, "Alpha"));
    assert!(!contains(&report.pending_backlink_scenarios, "Alpha"));
    assert!(
        report.unresolved_backlinks.is_empty(),
        "the backlink resolves against the change scenario"
    );
}

/// @spec audit/change-progress Pending scenarios do not fail the audit: A change with only pending scenarios reports no errors
#[test]
fn change_with_only_pending_scenarios_reports_no_errors() {
    let report = scoped_report(
        &["Alpha", "Beta"],
        &[("Alpha", false), ("Beta", false)],
        &[],
    );

    assert_eq!(
        report.total_errors(),
        0,
        "a change whose unlinked scenarios are all pending has no errors"
    );
    assert!(
        !report.pending_backlink_scenarios.is_empty(),
        "the pending scenarios are still reported"
    );
}

/// @spec audit/change-progress Pending scenarios do not fail the audit: A checked-but-unlinked scenario makes the audit report an error
#[test]
fn checked_but_unlinked_scenario_makes_the_audit_report_an_error() {
    let report = scoped_report(&["Alpha"], &[("Alpha", true)], &[]);

    assert!(
        report.total_errors() >= 1,
        "a claimed-but-unlinked scenario is counted as an error"
    );
}

/// @spec audit/change-progress Classification is scoped to the change audit: Full audit reports an unlinked caps scenario as an error, not pending
#[test]
fn full_audit_reports_unlinked_caps_scenario_as_error_not_pending() {
    let project = tempfile::tempdir().unwrap();
    let duckspec = project.path().join("duckspec");

    // A main-caps test:code scenario with no source backlink anywhere.
    write(&duckspec.join("caps/foo/spec.md"), &change_spec(&["Alpha"]));

    let config = Config::load(&duckspec).unwrap();
    let report = audit::run_audit(&duckspec, project.path(), &config, AuditScope::Full)
        .expect("audit runs");

    assert!(
        contains(&report.missing_backlink_scenarios, "Alpha"),
        "a full audit treats an unlinked caps scenario as an error"
    );
    assert!(
        report.pending_backlink_scenarios.is_empty(),
        "a full audit never produces pending scenarios"
    );
}
