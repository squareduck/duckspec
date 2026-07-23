//! Discover Codex stage skills under `.agents/skills/*/SKILL.md`.

use std::path::Path;

use crate::provider::SlashCommand;

/// List slash commands from project `.agents/skills` skill directories.
///
/// Each subdirectory that contains a `SKILL.md` with a usable name contributes
/// one command. Missing trees yield an empty list without error.
pub fn discover_commands(project_root: &Path) -> Vec<SlashCommand> {
    let skills_dir = project_root.join(".agents/skills");
    if !skills_dir.is_dir() {
        return Vec::new();
    }

    let mut commands = Vec::new();
    let entries = match std::fs::read_dir(&skills_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_file = path.join("SKILL.md");
        if !skill_file.is_file() {
            continue;
        }
        let dir_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if dir_name.is_empty() {
            continue;
        }
        let front = parse_skill_frontmatter(&skill_file);
        let name = front.name.filter(|n| !n.is_empty()).unwrap_or(dir_name);
        let description = front.description.unwrap_or_default();
        commands.push(SlashCommand { name, description });
    }

    commands.sort_by(|a, b| a.name.cmp(&b.name));
    commands.dedup_by(|a, b| a.name == b.name);
    commands
}

struct SkillFrontmatter {
    name: Option<String>,
    description: Option<String>,
}

fn parse_skill_frontmatter(path: &Path) -> SkillFrontmatter {
    let Ok(content) = std::fs::read_to_string(path) else {
        return SkillFrontmatter {
            name: None,
            description: None,
        };
    };
    let Some(body) = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
    else {
        return SkillFrontmatter {
            name: None,
            description: None,
        };
    };
    let Some(end) = body.find("\n---") else {
        return SkillFrontmatter {
            name: None,
            description: None,
        };
    };
    let frontmatter = &body[..end];
    let mut name = None;
    let mut description = None;
    for line in frontmatter.lines() {
        if let Some(v) = line.strip_prefix("name:") {
            let v = v.trim().trim_matches('"');
            if !v.is_empty() {
                name = Some(v.to_string());
            }
        } else if let Some(v) = line.strip_prefix("description:") {
            let v = v.trim().trim_matches('"');
            if !v.is_empty() {
                description = Some(v.to_string());
            }
        }
    }
    SkillFrontmatter { name, description }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// @spec harness/openai-codex Stage skill discovery: Skills under .agents/skills are listed as slash commands
    #[test]
    fn skills_under_agents_skills_are_listed() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join(".agents/skills/ds-propose");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: ds-propose\ndescription: Draft a proposal.\n---\n\nRun ds.\n",
        )
        .unwrap();
        let skill_dir2 = dir.path().join(".agents/skills/ds-explore");
        fs::create_dir_all(&skill_dir2).unwrap();
        fs::write(
            skill_dir2.join("SKILL.md"),
            "---\nname: ds-explore\ndescription: Explore the project.\n---\n\nRun explore.\n",
        )
        .unwrap();

        let cmds = discover_commands(dir.path());
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].name, "ds-explore");
        assert_eq!(cmds[1].name, "ds-propose");
        assert!(cmds.iter().any(|c| c.description.contains("proposal")));
    }

    /// @spec harness/openai-codex Stage skill discovery: A project without .agents/skills yields an empty command list
    #[test]
    fn project_without_agents_skills_yields_empty_list() {
        let dir = tempdir().unwrap();
        let cmds = discover_commands(dir.path());
        assert!(cmds.is_empty());
    }
}
