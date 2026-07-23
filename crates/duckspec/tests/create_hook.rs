//! End-to-end tests for `ds create hook` before/after scaffolding.

use std::fs;
use std::path::Path;
use std::process::Command;

fn empty_project() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    // Minimal duckspec root so `find_duckspec_root` succeeds.
    fs::create_dir_all(project.path().join("duckspec")).unwrap();
    project
}

fn ds(project_root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ds"))
        .args(args)
        .current_dir(project_root)
        .output()
        .expect("run ds")
}

// @spec cli/hooks Create hook: Scaffold before and after paths with skeleton
#[test]
fn scaffold_before_and_after_paths_with_skeleton() {
    let project = empty_project();
    let root = project.path();

    let before = ds(root, &["create", "hook", "explore", "--before"]);
    assert!(
        before.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&before.stderr)
    );
    let after = ds(root, &["create", "hook", "explore", "--after"]);
    assert!(
        after.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&after.stderr)
    );

    let before_path = root.join("duckspec/hooks/explore-before.md");
    let after_path = root.join("duckspec/hooks/explore-after.md");
    assert_eq!(
        fs::read_to_string(&before_path).unwrap(),
        "# Explore - Before\n"
    );
    assert_eq!(
        fs::read_to_string(&after_path).unwrap(),
        "# Explore - After\n"
    );
}

// @spec cli/hooks Create hook: Refuse without exactly one of before or after
#[test]
fn refuse_without_exactly_one_of_before_or_after() {
    let project = empty_project();
    let root = project.path();

    let neither = ds(root, &["create", "hook", "explore"]);
    assert!(!neither.status.success());
    assert!(!root.join("duckspec/hooks/explore-before.md").exists());
    assert!(!root.join("duckspec/hooks/explore-after.md").exists());

    let both = ds(root, &["create", "hook", "explore", "--before", "--after"]);
    assert!(!both.status.success());
    assert!(!root.join("duckspec/hooks/explore-before.md").exists());
    assert!(!root.join("duckspec/hooks/explore-after.md").exists());
}
