use std::fs;
use std::path::Path;

use clap::Subcommand;
use owo_colors::OwoColorize;

use super::common::{find_duckspec_root, list_subdirs};

#[derive(Subcommand, Debug)]
pub enum CreateCommand {
    /// Create a new change directory.
    Change {
        /// Name for the new change.
        name: String,
    },
    /// Create proposal.md in a change.
    Proposal {
        /// Change to create the proposal in.
        #[arg(long = "in")]
        change: String,
    },
    /// Create design.md in a change.
    Design {
        /// Change to create the design in.
        #[arg(long = "in")]
        change: String,
    },
    /// Create a spec file (full or delta) for a capability in a change.
    Spec {
        /// Capability path (e.g. auth/google).
        cap_path: String,
        /// Change to create the spec in.
        #[arg(long = "in")]
        change: String,
    },
    /// Create a doc file (full or delta) for a capability in a change.
    Doc {
        /// Capability path (e.g. auth/google).
        cap_path: String,
        /// Change to create the doc in.
        #[arg(long = "in")]
        change: String,
    },
    /// Create a step file in a change.
    Step {
        /// Name for the step (will be slugified).
        name: String,
        /// Change to create the step in.
        #[arg(long = "in")]
        change: String,
        /// Insert after this step slug (triggers renumbering).
        #[arg(long)]
        after: Option<String>,
    },
    /// Create a hook file for a stage.
    Hook {
        /// Stage name (explore, propose, design, spec, step, apply, archive, verify, codex).
        stage: String,
        /// Create a pre-stage hook.
        #[arg(long, group = "position")]
        pre: bool,
        /// Create a post-stage hook.
        #[arg(long, group = "position")]
        post: bool,
    },
}

pub fn run(cmd: CreateCommand) -> anyhow::Result<()> {
    let duckspec_root = find_duckspec_root()?;

    // Capture hook info before cmd is consumed.
    let hook_content = if let CreateCommand::Hook { ref stage, pre, .. } = cmd {
        let pos = if pre { "Pre" } else { "Post" };
        let title = capitalize(stage);
        Some(format!("# {title} - {pos}\n"))
    } else {
        None
    };

    let plan = match cmd {
        CreateCommand::Change { name } => {
            let active = list_subdirs(&duckspec_root.join("changes"))?;
            let archived = list_subdirs(&duckspec_root.join("archive"))?;
            duckpond::plan::create_change(&name, &active, &archived)?
        }
        CreateCommand::Proposal { change } => {
            let active = list_subdirs(&duckspec_root.join("changes"))?;
            let change_files = list_files(&duckspec_root.join("changes").join(&change))?;
            duckpond::plan::create_proposal(&change, &active, &change_files)?
        }
        CreateCommand::Design { change } => {
            let active = list_subdirs(&duckspec_root.join("changes"))?;
            let change_files = list_files(&duckspec_root.join("changes").join(&change))?;
            duckpond::plan::create_design(&change, &active, &change_files)?
        }
        CreateCommand::Spec { cap_path, change } => {
            let active = list_subdirs(&duckspec_root.join("changes"))?;
            let toplevel_caps = collect_cap_paths(&duckspec_root.join("caps"))?;
            let change_cap_dir = duckspec_root
                .join("changes")
                .join(&change)
                .join("caps")
                .join(&cap_path);
            let change_cap_files = list_files(&change_cap_dir)?;
            duckpond::plan::create_spec(
                &cap_path,
                &change,
                &active,
                &toplevel_caps,
                &change_cap_files,
            )?
        }
        CreateCommand::Doc { cap_path, change } => {
            let active = list_subdirs(&duckspec_root.join("changes"))?;
            let toplevel_caps = collect_cap_paths(&duckspec_root.join("caps"))?;
            let change_cap_dir = duckspec_root
                .join("changes")
                .join(&change)
                .join("caps")
                .join(&cap_path);
            let change_cap_files = list_files(&change_cap_dir)?;
            duckpond::plan::create_doc(
                &cap_path,
                &change,
                &active,
                &toplevel_caps,
                &change_cap_files,
            )?
        }
        CreateCommand::Step {
            name,
            change,
            after,
        } => {
            let active = list_subdirs(&duckspec_root.join("changes"))?;
            let steps_dir = duckspec_root.join("changes").join(&change).join("steps");
            let existing_steps = list_files(&steps_dir)?;
            duckpond::plan::create_step(&name, &change, &active, &existing_steps, after.as_deref())?
        }
        CreateCommand::Hook { stage, pre, post } => {
            let position = if pre {
                duckpond::plan::HookPosition::Pre
            } else if post {
                duckpond::plan::HookPosition::Post
            } else {
                anyhow::bail!("exactly one of --pre or --post must be provided");
            };
            let existing_hooks = list_files(&duckspec_root.join("hooks"))?;
            duckpond::plan::create_hook(&stage, position, &existing_hooks)?
        }
    };

    // Execute: renames first (already in safe order), then creates.
    for (from, to) in &plan.renames {
        let abs_from = duckspec_root.join(from);
        let abs_to = duckspec_root.join(to);
        fs::rename(&abs_from, &abs_to)?;
    }
    for path in &plan.creates {
        let abs = duckspec_root.join(path);
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }
        if abs.extension().is_some() {
            // It's a file — write hook skeleton, or an H1 placeholder
            // for artifacts so editors that require a non-empty Read
            // before Write don't trip on a fresh create.
            let filename = abs.file_name().and_then(|f| f.to_str()).unwrap_or_default();
            let content = hook_content
                .clone()
                .unwrap_or_else(|| placeholder_for(filename));
            fs::write(&abs, content)?;
        } else {
            // It's a directory.
            fs::create_dir_all(&abs)?;
        }
    }

    // Display what was done.
    for (from, to) in &plan.renames {
        println!(
            "  {} {} → {}",
            "renamed".dimmed(),
            from.display(),
            to.display()
        );
    }
    for path in &plan.creates {
        println!("  {} {}", "created".green(), path.display());
    }

    Ok(())
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

/// Return the H1 placeholder content for a freshly-created artifact file.
///
/// The LLM is expected to replace this with the real H1 (typically a
/// title) when it populates the file. The placeholder exists only so
/// that tools requiring a non-empty Read before Write don't trip on a
/// just-created file.
fn placeholder_for(filename: &str) -> String {
    let title = match filename {
        "proposal.md" => "Proposal",
        "design.md" => "Design",
        "spec.md" => "Spec",
        "spec.delta.md" => "Spec Delta",
        "doc.md" => "Doc",
        "doc.delta.md" => "Doc Delta",
        _ if is_step_filename(filename) => "Step",
        _ => return String::new(),
    };
    format!("# {title}\n")
}

/// Step files are named `NN-<slug>.md` where `NN` is two digits.
fn is_step_filename(filename: &str) -> bool {
    let bytes = filename.as_bytes();
    bytes.len() >= 4
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b'-'
        && filename.ends_with(".md")
}

/// List filenames (not directories) directly in a directory.
/// Returns an empty vec if the directory doesn't exist.
fn list_files(dir: &Path) -> anyhow::Result<Vec<String>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().is_file()
            && let Some(name) = entry.file_name().to_str()
        {
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names)
}

/// Recursively collect capability paths from the top-level `caps/` directory.
/// A capability exists where `spec.md` is found.
fn collect_cap_paths(caps_dir: &Path) -> anyhow::Result<Vec<String>> {
    let mut paths = Vec::new();
    if caps_dir.is_dir() {
        scan_caps(caps_dir, caps_dir, &mut paths)?;
    }
    Ok(paths)
}

fn scan_caps(dir: &Path, caps_root: &Path, out: &mut Vec<String>) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            scan_caps(&path, caps_root, out)?;
        } else if path.file_name().and_then(|f| f.to_str()) == Some("spec.md")
            && let Some(parent) = path.parent()
            && let Ok(rel) = parent.strip_prefix(caps_root)
        {
            out.push(rel.to_string_lossy().into_owned());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_for_named_artifacts() {
        assert_eq!(placeholder_for("proposal.md"), "# Proposal\n");
        assert_eq!(placeholder_for("design.md"), "# Design\n");
        assert_eq!(placeholder_for("spec.md"), "# Spec\n");
        assert_eq!(placeholder_for("spec.delta.md"), "# Spec Delta\n");
        assert_eq!(placeholder_for("doc.md"), "# Doc\n");
        assert_eq!(placeholder_for("doc.delta.md"), "# Doc Delta\n");
    }

    #[test]
    fn placeholder_for_step_files() {
        assert_eq!(placeholder_for("01-add-validate.md"), "# Step\n");
        assert_eq!(placeholder_for("12-x.md"), "# Step\n");
    }

    #[test]
    fn placeholder_for_unknown_returns_empty() {
        assert_eq!(placeholder_for("notes.md"), "");
        assert_eq!(placeholder_for("README.md"), "");
        assert_eq!(placeholder_for("1-missing-leading-zero.md"), "");
    }
}
