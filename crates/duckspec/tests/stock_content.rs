//! End-to-end tests for stock CLI content (`template`, `schema`, `init`).

use std::fs;
use std::path::Path;
use std::process::Command;

use include_dir::{Dir, include_dir};

/// Same stock tree the binary embeds — used to assert installed bodies match.
static STOCK_COMMANDS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/content/commands");

fn ds(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ds"))
        .args(args)
        .output()
        .expect("run ds")
}

fn ds_in(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ds"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run ds")
}

/// @spec cli/stock-content Stock content from the binary: Known template is printed
#[test]
fn known_template_is_printed() {
    let output = ds(&["template", "explore"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "template explore must succeed; stderr:\n{stderr}"
    );
    assert!(
        stdout.contains("# explore"),
        "stdout must contain stock explore template; got:\n{stdout}"
    );
}

/// @spec cli/stock-content Stock content from the binary: Known schema is printed
#[test]
fn known_schema_is_printed() {
    let output = ds(&["schema", "proposal"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "schema proposal must succeed; stderr:\n{stderr}"
    );
    assert!(
        stdout.contains("# Proposal schema") || stdout.contains("Proposal"),
        "stdout must contain stock proposal schema; got:\n{stdout}"
    );
}

/// @spec cli/stock-content Clear unknown-name failures: Unknown template is rejected by name
#[test]
fn unknown_template_is_rejected_by_name() {
    let output = ds(&["template", "not-a-real-template"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stderr}{}", String::from_utf8_lossy(&output.stdout));

    assert!(
        !output.status.success(),
        "unknown template must fail; stderr:\n{stderr}"
    );
    assert!(
        combined.contains("not-a-real-template") && combined.to_lowercase().contains("unknown"),
        "error must identify the unknown template name; got:\n{combined}"
    );
    assert!(
        !combined.contains("No such file or directory")
            && !combined.contains("os error 2")
            && !combined.contains("content/templates"),
        "error must not be a filesystem path miss for stock content; got:\n{combined}"
    );
}

/// @spec cli/stock-content Clear unknown-name failures: Unknown schema is rejected by name
#[test]
fn unknown_schema_is_rejected_by_name() {
    let output = ds(&["schema", "not-a-real-schema"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stderr}{}", String::from_utf8_lossy(&output.stdout));

    assert!(
        !output.status.success(),
        "unknown schema must fail; stderr:\n{stderr}"
    );
    assert!(
        combined.contains("not-a-real-schema") && combined.to_lowercase().contains("unknown"),
        "error must identify the unknown schema name; got:\n{combined}"
    );
    assert!(
        !combined.contains("No such file or directory")
            && !combined.contains("os error 2")
            && !combined.contains("content/schemas"),
        "error must not be a filesystem path miss for stock content; got:\n{combined}"
    );
}

/// @spec cli/stock-content Stock content from the binary: Known harness commands are installed under the harness path
#[test]
fn known_harness_commands_are_installed_under_the_harness_path() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();

    let output = ds_in(root, &["init", "claude"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "init claude must succeed; stderr:\n{stderr}"
    );

    let stock = STOCK_COMMANDS
        .get_dir("claude")
        .expect("stock claude commands present at compile time");
    let target = root.join(".claude/commands");
    assert!(
        target.is_dir(),
        ".claude/commands must exist after init claude"
    );

    let mut installed = 0usize;
    for file in stock.files() {
        let name = file
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .expect("utf-8 filename");
        if !name.ends_with(".md") {
            continue;
        }
        installed += 1;
        let dest = target.join(name);
        let body = fs::read_to_string(&dest).unwrap_or_else(|e| {
            panic!("expected installed {name}: {e}");
        });
        let expected = file.contents_utf8().expect("stock command is valid UTF-8");
        assert_eq!(
            body, expected,
            "installed {name} must match stock command body"
        );
    }
    assert!(
        installed > 0,
        "expected at least one stock claude command file"
    );
}

/// @spec cli/stock-content Clear unknown-name failures: Unknown harness is rejected by name
#[test]
fn unknown_harness_is_rejected_by_name() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();

    let output = ds_in(root, &["init", "not-a-real-harness"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stderr}{}", String::from_utf8_lossy(&output.stdout));

    assert!(
        !output.status.success(),
        "unknown harness must fail; stderr:\n{stderr}"
    );
    assert!(
        combined.contains("not-a-real-harness") && combined.to_lowercase().contains("unknown"),
        "error must identify the unknown harness name; got:\n{combined}"
    );
    assert!(
        !combined.contains("No such file or directory")
            && !combined.contains("os error 2")
            && !combined.contains("content/commands"),
        "error must not be a filesystem path miss for stock content; got:\n{combined}"
    );
}

/// @spec cli/stock-content Stock content from the binary: Known codex skills are installed under .agents/skills
#[test]
fn known_codex_skills_are_installed_under_agents_skills() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();

    let output = ds_in(root, &["init", "codex"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "init codex must succeed; stderr:\n{stderr}"
    );

    let stock = STOCK_COMMANDS
        .get_dir("codex")
        .expect("stock codex skills present at compile time");
    let target = root.join(".agents/skills");
    assert!(
        target.is_dir(),
        ".agents/skills must exist after init codex"
    );

    let mut installed = 0usize;
    for skill_dir in stock.dirs() {
        let dir_name = skill_dir
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .expect("utf-8 skill dir name");
        // Paths in include_dir are embed-root-relative; match by basename.
        let skill_md = skill_dir
            .files()
            .find(|f| f.path().file_name().and_then(|n| n.to_str()) == Some("SKILL.md"))
            .expect("stock skill has SKILL.md");
        installed += 1;
        let dest = target.join(dir_name).join("SKILL.md");
        let body = fs::read_to_string(&dest).unwrap_or_else(|e| {
            panic!("expected installed {dir_name}/SKILL.md: {e}");
        });
        let expected = skill_md
            .contents_utf8()
            .expect("stock skill is valid UTF-8");
        assert_eq!(
            body, expected,
            "installed {dir_name}/SKILL.md must match stock skill body"
        );
    }
    assert!(
        installed > 0,
        "expected at least one stock codex skill directory"
    );

    // Re-init overwrites skill bodies.
    let propose = target.join("ds-propose/SKILL.md");
    fs::write(&propose, "stale body\n").unwrap();
    let re = ds_in(root, &["init", "codex"]);
    assert!(re.status.success(), "re-init codex must succeed");
    let restored = fs::read_to_string(&propose).unwrap();
    assert_ne!(restored, "stale body\n");
    assert!(restored.contains("ds template propose"));
}
