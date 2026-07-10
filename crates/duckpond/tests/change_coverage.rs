//! Integration tests for `duckpond::change_coverage::for_change`.
//!
//! Backlink-bearing fixture sources use inline `\n` escapes so this project's
//! own `@spec` scan does not treat them as live backlinks.

use std::fs;
use std::path::Path;

use duckpond::audit::ScenarioKey;
use duckpond::change_coverage::{self, ChangeCoverage};
use duckpond::config::Config;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

/// New-cap spec for capability `foo`, requirement `Behavior`, named scenarios.
/// When `with_marker_paths` is true each scenario gets an empty-looking path list
/// entry so marker paths alone are non-empty without a source backlink.
fn change_spec(scenarios: &[&str], with_marker_paths: bool) -> String {
    let mut s = String::from(
        "# Foo\n\nA new capability.\n\n## Requirement: Behavior\n\nThe system SHALL behave.\n\n> test: code\n",
    );
    for name in scenarios {
        s.push_str(&format!(
            "\n### Scenario: {name}\n\n- **WHEN** x happens\n- **THEN** y follows\n\n> test: code\n"
        ));
        if with_marker_paths {
            s.push_str("> - tests/phantom.rs:1\n");
        }
    }
    s
}

fn change_spec_manual(scenario: &str) -> String {
    format!(
        "# Foo\n\nA new capability.\n\n## Requirement: Behavior\n\nThe system SHALL behave.\n\n\
         ### Scenario: {scenario}\n\n- **WHEN** x happens\n- **THEN** y follows\n\n\
         > manual: visual check only\n"
    )
}

fn base_cap_spec(scenario: &str) -> String {
    format!(
        "# Foo\n\nBase capability.\n\n## Requirement: Behavior\n\nThe system SHALL behave.\n\n\
         > test: code\n\n### Scenario: {scenario}\n\n- **WHEN** x happens\n- **THEN** y follows\n"
    )
}

/// Delta that anchors Behavior and adds one new test:code scenario.
fn add_scenario_delta(new_scenario: &str) -> String {
    format!(
        "# @ Foo\n\n## @ Requirement: Behavior\n\n### + Scenario: {new_scenario}\n\n\
         - **WHEN** x happens\n- **THEN** y follows\n\n> test: code\n"
    )
}

/// Source backlink for `foo Behavior: <scenario>` (inline `\n`, not a real multi-line comment here).
fn backlink_source(scenario: &str) -> String {
    format!("// @spec foo Behavior: {scenario}\nfn t() {{}}\n")
}

fn step_body(scenario: &str, checked: bool) -> String {
    let mark = if checked { "x" } else { " " };
    format!(
        "# Implement\n\nDo the work.\n\n## Tasks\n\n- [{mark}] @spec foo Behavior: {scenario}\n"
    )
}

fn key_in(keys: &[ScenarioKey], scenario: &str) -> bool {
    keys.iter().any(|k| k.scenario == scenario && k.cap_path == "foo")
}

fn in_snapshot(cov: &ChangeCoverage, scenario: &str) -> bool {
    key_in(&cov.linked, scenario) || key_in(&cov.open, scenario)
}

/// Project + optional source backlinks for change `add-foo` with a new-cap spec.
fn coverage_new_cap(
    scenarios: &[&str],
    with_marker_paths: bool,
    backlinked: &[&str],
    step: Option<(&str, bool)>,
) -> ChangeCoverage {
    let project = tempfile::tempdir().unwrap();
    let duckspec = project.path().join("duckspec");

    write(
        &duckspec.join("changes/add-foo/caps/foo/spec.md"),
        &change_spec(scenarios, with_marker_paths),
    );
    if let Some((scn, checked)) = step {
        write(
            &duckspec.join("changes/add-foo/steps/01-implement.md"),
            &step_body(scn, checked),
        );
    }
    for (i, scenario) in backlinked.iter().enumerate() {
        write(
            &project.path().join(format!("tests/foo_{i}.rs")),
            &backlink_source(scenario),
        );
    }

    let config = Config::load(&duckspec).unwrap();
    change_coverage::for_change(&duckspec, project.path(), &config, "add-foo")
        .expect("for_change runs")
}

// ---------------------------------------------------------------------------
// Source backlink is the linkage signal
// ---------------------------------------------------------------------------

/// @spec status/change-coverage Source backlink is the linkage signal: Resolving source backlink makes the scenario linked
#[test]
fn resolving_source_backlink_makes_the_scenario_linked() {
    let cov = coverage_new_cap(&["Alpha"], false, &["Alpha"], None);

    assert!(
        key_in(&cov.linked, "Alpha"),
        "expected Alpha linked, got linked={:?} open={:?}",
        cov.linked.iter().map(|k| k.display()).collect::<Vec<_>>(),
        cov.open.iter().map(|k| k.display()).collect::<Vec<_>>()
    );
}

/// @spec status/change-coverage Source backlink is the linkage signal: Marker path list without a source backlink leaves the scenario open
#[test]
fn marker_path_list_without_source_backlink_leaves_scenario_open() {
    let cov = coverage_new_cap(&["Alpha"], true, &[], None);

    assert!(
        key_in(&cov.open, "Alpha"),
        "marker paths alone must not link; open={:?} linked={:?}",
        cov.open.iter().map(|k| k.display()).collect::<Vec<_>>(),
        cov.linked.iter().map(|k| k.display()).collect::<Vec<_>>()
    );
    assert!(!key_in(&cov.linked, "Alpha"));
}

/// @spec status/change-coverage Source backlink is the linkage signal: A linked scenario is not reported as open
#[test]
fn linked_scenario_is_not_reported_as_open() {
    let cov = coverage_new_cap(&["Alpha"], false, &["Alpha"], None);

    assert!(key_in(&cov.linked, "Alpha"));
    assert!(
        !key_in(&cov.open, "Alpha"),
        "linked scenario must not appear in open"
    );
}

// ---------------------------------------------------------------------------
// Snapshot is change-introduced test code only
// ---------------------------------------------------------------------------

/// @spec status/change-coverage Snapshot is change-introduced test code only: New change-cap test:code scenario is included
#[test]
fn new_change_cap_test_code_scenario_is_included() {
    let cov = coverage_new_cap(&["Alpha"], false, &[], None);

    assert!(
        in_snapshot(&cov, "Alpha"),
        "new-cap test:code scenario must appear in the snapshot"
    );
}

/// @spec status/change-coverage Snapshot is change-introduced test code only: Delta-introduced test:code scenario is included
#[test]
fn delta_introduced_test_code_scenario_is_included() {
    let project = tempfile::tempdir().unwrap();
    let duckspec = project.path().join("duckspec");

    write(
        &duckspec.join("caps/foo/spec.md"),
        &base_cap_spec("Existing"),
    );
    write(
        &duckspec.join("changes/add-foo/caps/foo/spec.delta.md"),
        &add_scenario_delta("NewOne"),
    );

    let config = Config::load(&duckspec).unwrap();
    let cov = change_coverage::for_change(&duckspec, project.path(), &config, "add-foo")
        .expect("for_change runs");

    assert!(
        in_snapshot(&cov, "NewOne"),
        "delta-introduced scenario must be in snapshot; linked={:?} open={:?}",
        cov.linked.iter().map(|k| k.display()).collect::<Vec<_>>(),
        cov.open.iter().map(|k| k.display()).collect::<Vec<_>>()
    );
}

/// @spec status/change-coverage Snapshot is change-introduced test code only: Pre-existing base scenario is excluded
#[test]
fn pre_existing_base_scenario_is_excluded() {
    let project = tempfile::tempdir().unwrap();
    let duckspec = project.path().join("duckspec");

    write(
        &duckspec.join("caps/foo/spec.md"),
        &base_cap_spec("Existing"),
    );
    // Delta adds a different scenario; Existing is only in base.
    write(
        &duckspec.join("changes/add-foo/caps/foo/spec.delta.md"),
        &add_scenario_delta("NewOne"),
    );

    let config = Config::load(&duckspec).unwrap();
    let cov = change_coverage::for_change(&duckspec, project.path(), &config, "add-foo")
        .expect("for_change runs");

    assert!(
        !in_snapshot(&cov, "Existing"),
        "base-only scenario must not appear in change snapshot"
    );
    assert!(in_snapshot(&cov, "NewOne"));
}

/// @spec status/change-coverage Snapshot is change-introduced test code only: Non-test:code scenario is excluded
#[test]
fn non_test_code_scenario_is_excluded() {
    let project = tempfile::tempdir().unwrap();
    let duckspec = project.path().join("duckspec");

    write(
        &duckspec.join("changes/add-foo/caps/foo/spec.md"),
        &change_spec_manual("VisualOnly"),
    );

    let config = Config::load(&duckspec).unwrap();
    let cov = change_coverage::for_change(&duckspec, project.path(), &config, "add-foo")
        .expect("for_change runs");

    assert!(
        !in_snapshot(&cov, "VisualOnly"),
        "manual: scenario must not enter the test:code snapshot"
    );
    assert!(cov.linked.is_empty() && cov.open.is_empty());
}

// ---------------------------------------------------------------------------
// Step checkbox independence
// ---------------------------------------------------------------------------

/// @spec status/change-coverage Change status surfaces the partition: Step checkbox state does not change linkage
#[test]
fn step_checkbox_state_does_not_change_linkage() {
    let unchecked = coverage_new_cap(&["Alpha"], false, &[], Some(("Alpha", false)));
    let checked = coverage_new_cap(&["Alpha"], false, &[], Some(("Alpha", true)));

    assert!(
        key_in(&unchecked.open, "Alpha") && !key_in(&unchecked.linked, "Alpha"),
        "unchecked step: unlinked scenario is open"
    );
    assert!(
        key_in(&checked.open, "Alpha") && !key_in(&checked.linked, "Alpha"),
        "checked step: unlinked scenario is still open (not claimed as linked)"
    );
    assert_eq!(
        unchecked.open.len(),
        checked.open.len(),
        "checkbox state must not change open membership"
    );
    assert_eq!(unchecked.linked.len(), checked.linked.len());
}
