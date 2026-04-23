//! Discovery of project-level and plugin-level slash commands / skills that
//! `claude` will accept.

use std::path::{Path, PathBuf};

use crate::provider::SlashCommand;

pub fn discover_commands(project_root: &Path) -> Vec<SlashCommand> {
    let mut commands = Vec::new();

    let project_cmds = project_root.join(".claude/commands");
    if project_cmds.is_dir() {
        scan_command_dir(&project_cmds, &mut commands);
    }

    if let Ok(home) = std::env::var("HOME") {
        let claude_dir = PathBuf::from(home).join(".claude");
        let settings_path = claude_dir.join("settings.json");
        if let Ok(settings_str) = std::fs::read_to_string(&settings_path)
            && let Ok(settings) = serde_json::from_str::<serde_json::Value>(&settings_str)
            && let Some(plugins) = settings["enabledPlugins"].as_object()
        {
            for (key, enabled) in plugins {
                if enabled.as_bool() != Some(true) {
                    continue;
                }
                if let Some((plugin_name, marketplace)) = key.rsplit_once('@') {
                    let plugin_dir = claude_dir
                        .join("plugins/marketplaces")
                        .join(marketplace)
                        .join("plugins")
                        .join(plugin_name);
                    let cmd_dir = plugin_dir.join("commands");
                    if cmd_dir.is_dir() {
                        scan_command_dir(&cmd_dir, &mut commands);
                    }
                    let skills_dir = plugin_dir.join("skills");
                    if skills_dir.is_dir() {
                        scan_skills_dir(&skills_dir, &mut commands);
                    }
                }
            }
        }
    }

    // Built-in Claude Code commands (not discoverable from filesystem).
    let builtins = [
        ("clear", "Clear conversation history"),
        ("compact", "Summarize and compact conversation"),
        ("cost", "Show token usage and cost"),
        ("help", "Show available commands"),
        ("model", "Switch the model"),
    ];
    for (name, desc) in builtins {
        commands.push(SlashCommand {
            name: name.into(),
            description: desc.into(),
        });
    }

    commands.sort_by(|a, b| a.name.cmp(&b.name));
    commands.dedup_by(|a, b| a.name == b.name);
    commands
}

fn scan_command_dir(dir: &Path, commands: &mut Vec<SlashCommand>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let description = parse_frontmatter_description(&path).unwrap_or_default();
        commands.push(SlashCommand { name, description });
    }
}

fn scan_skills_dir(dir: &Path, commands: &mut Vec<SlashCommand>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_file = path.join("SKILL.md");
        if !skill_file.exists() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let description = parse_frontmatter_description(&skill_file).unwrap_or_default();
        commands.push(SlashCommand { name, description });
    }
}

fn parse_frontmatter_description(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let body = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))?;
    let end = body.find("\n---")?;
    let frontmatter = &body[..end];
    for line in frontmatter.lines() {
        if let Some(desc) = line.strip_prefix("description:") {
            let desc = desc.trim().trim_matches('"');
            if !desc.is_empty() {
                return Some(desc.to_string());
            }
        }
    }
    None
}
