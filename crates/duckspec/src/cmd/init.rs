use std::fs;
use std::path::Path;

use owo_colors::OwoColorize;

use crate::content;

const DUCKSPEC_SUBDIRS: &[&str] = &["archive", "caps", "codex", "changes"];

/// How stock commands are laid out under `content/commands/{harness}/`.
#[derive(Clone, Copy)]
enum InstallLayout {
    /// Flat `ds-*.md` files → target dir (claude / opencode).
    FlatMarkdown,
    /// Skill directories with `SKILL.md` → target dir (codex → `.agents/skills`).
    SkillDirs,
}

const HARNESS_COMMAND_DIR: &[(&str, &str, InstallLayout)] = &[
    ("claude", ".claude/commands", InstallLayout::FlatMarkdown),
    ("opencode", ".opencode/commands", InstallLayout::FlatMarkdown),
    ("codex", ".agents/skills", InstallLayout::SkillDirs),
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
        let (target_rel, layout) = HARNESS_COMMAND_DIR
            .iter()
            .find(|(name, _, _)| *name == harness_name)
            .map(|(_, dir, layout)| (*dir, *layout))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "unknown harness: {harness_name} (supported: {})",
                    supported_harnesses().join(", ")
                )
            })?;

        if !content::has_harness(&harness_name) {
            anyhow::bail!(
                "unknown harness: {harness_name} (no stock content; supported: {})",
                supported_harnesses().join(", ")
            );
        }

        let target_dir = cwd.join(target_rel);
        fs::create_dir_all(&target_dir)?;
        install_commands(&harness_name, &target_dir, layout)?;
    }

    Ok(())
}

fn supported_harnesses() -> Vec<&'static str> {
    HARNESS_COMMAND_DIR
        .iter()
        .map(|(name, _, _)| *name)
        .collect()
}

fn install_commands(
    harness: &str,
    target_dir: &Path,
    layout: InstallLayout,
) -> anyhow::Result<()> {
    match layout {
        InstallLayout::FlatMarkdown => {
            for (filename, body) in content::command_files(harness) {
                let dest = target_dir.join(filename);
                fs::write(&dest, body)?;
                println!("  {} {}", "installed".green(), dest.display());
            }
        }
        InstallLayout::SkillDirs => {
            for (dir_name, body) in content::skill_dirs(harness) {
                let skill_dir = target_dir.join(dir_name);
                fs::create_dir_all(&skill_dir)?;
                let dest = skill_dir.join("SKILL.md");
                fs::write(&dest, body)?;
                println!("  {} {}", "installed".green(), dest.display());
            }
        }
    }
    Ok(())
}
