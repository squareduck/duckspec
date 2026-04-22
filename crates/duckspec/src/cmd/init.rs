use std::fs;
use std::path::Path;

use owo_colors::OwoColorize;

const COMMANDS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/content/commands");

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

        let source_dir = format!("{COMMANDS_DIR}/{harness_name}");
        let target_dir = cwd.join(target_rel);
        fs::create_dir_all(&target_dir)?;

        install_commands(Path::new(&source_dir), &target_dir)?;
    }

    Ok(())
}

fn install_commands(source_dir: &Path, target_dir: &Path) -> anyhow::Result<()> {
    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let filename = entry.file_name();
            let dest = target_dir.join(&filename);
            fs::copy(&path, &dest)?;
            println!("  {} {}", "installed".green(), dest.display());
        }
    }
    Ok(())
}
