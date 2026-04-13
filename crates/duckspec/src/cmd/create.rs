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
}

pub fn run(cmd: CreateCommand) -> anyhow::Result<()> {
    let duckspec_root = find_duckspec_root()?;

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
            let steps_dir = duckspec_root
                .join("changes")
                .join(&change)
                .join("steps");
            let existing_steps = list_files(&steps_dir)?;
            duckpond::plan::create_step(
                &name,
                &change,
                &active,
                &existing_steps,
                after.as_deref(),
            )?
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
            // It's a file — create empty.
            fs::write(&abs, "")?;
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

/// List filenames (not directories) directly in a directory.
/// Returns an empty vec if the directory doesn't exist.
fn list_files(dir: &Path) -> anyhow::Result<Vec<String>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().is_file() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
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
        } else if path.file_name().and_then(|f| f.to_str()) == Some("spec.md") {
            if let Some(parent) = path.parent() {
                if let Ok(rel) = parent.strip_prefix(caps_root) {
                    out.push(rel.to_string_lossy().into_owned());
                }
            }
        }
    }
    Ok(())
}
