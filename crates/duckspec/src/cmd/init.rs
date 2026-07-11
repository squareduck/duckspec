use std::fs;
use std::path::Path;

use owo_colors::OwoColorize;

use crate::content;

const DUCKSPEC_SUBDIRS: &[&str] = &["archive", "caps", "codex", "changes"];

const HARNESS_COMMAND_DIR: &[(&str, &str)] = &[
    ("claude", ".claude/commands"),
    ("opencode", ".opencode/commands"),
];

pub fn run(harness: Option<String>) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let duckspec = cwd.join("duckspec");

    // Create duckspec/ and subdirectories (idempotent).
    for subdir in DUCKSPEC_SUBDIRS {
        let dir = duckspec.join(subdir);
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
            println!("  {} duckspec/{subdir}/", "created".green());
        }
    }

    // Install harness commands if requested.
    if let Some(harness_name) = harness {
        let target_rel = HARNESS_COMMAND_DIR
            .iter()
            .find(|(name, _)| *name == harness_name)
            .map(|(_, dir)| *dir)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "unknown harness: {harness_name} (supported: {})",
                    HARNESS_COMMAND_DIR
                        .iter()
                        .map(|(name, _)| *name)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;

        let target_dir = cwd.join(target_rel);
        fs::create_dir_all(&target_dir)?;
        install_commands(&harness_name, &target_dir)?;
    }

    Ok(())
}

fn install_commands(harness: &str, target_dir: &Path) -> anyhow::Result<()> {
    for (filename, body) in content::command_files(harness) {
        let dest = target_dir.join(filename);
        fs::write(&dest, body)?;
        println!("  {} {}", "installed".green(), dest.display());
    }
    Ok(())
}
