//! Stock CLI content embedded in the `ds` binary at compile time.
//!
//! Source lives under `crates/duckspec/content/`; runtime lookups never
//! touch the build-machine filesystem.

use include_dir::{Dir, include_dir};

static CONTENT: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/content");

fn utf8_file(rel: &str) -> Option<&'static str> {
    CONTENT
        .get_file(rel)
        .map(|f| f.contents_utf8().expect("stock content is valid UTF-8"))
}

/// UTF-8 body of `templates/{name}.md`, or `None` if missing.
pub fn template(name: &str) -> Option<&'static str> {
    utf8_file(&format!("templates/{name}.md"))
}

/// UTF-8 body of `schemas/{name}.md`, or `None` if missing.
pub fn schema(name: &str) -> Option<&'static str> {
    utf8_file(&format!("schemas/{name}.md"))
}

/// Whether `commands/{harness}/` exists in the embed.
pub fn has_harness(harness: &str) -> bool {
    CONTENT.get_dir(format!("commands/{harness}")).is_some()
}

/// Flat markdown files under `commands/{harness}/` (claude / opencode).
///
/// Each entry is `(file_name, utf8_body)`.
pub fn command_files(harness: &str) -> impl Iterator<Item = (&'static str, &'static str)> {
    let Some(dir) = CONTENT.get_dir(format!("commands/{harness}")) else {
        return Box::new(std::iter::empty())
            as Box<dyn Iterator<Item = (&'static str, &'static str)>>;
    };
    Box::new(dir.files().filter_map(|f| {
        let name = f.path().file_name()?.to_str()?;
        if !name.ends_with(".md") {
            return None;
        }
        let body = f.contents_utf8().expect("stock content is valid UTF-8");
        Some((name, body))
    }))
}

/// Skill directories under `commands/{harness}/` (codex layout).
///
/// Each entry is `(skill_dir_name, SKILL.md utf8 body)`. Only immediate child
/// directories that contain a `SKILL.md` are yielded.
pub fn skill_dirs(harness: &str) -> impl Iterator<Item = (&'static str, &'static str)> {
    let Some(dir) = CONTENT.get_dir(format!("commands/{harness}")) else {
        return Box::new(std::iter::empty())
            as Box<dyn Iterator<Item = (&'static str, &'static str)>>;
    };
    // include_dir stores file paths relative to the embed root (e.g.
    // `commands/codex/ds-propose/SKILL.md`), so look up by basename via
    // `files()` rather than `get_file("SKILL.md")`.
    Box::new(dir.dirs().filter_map(|skill_dir| {
        let name = skill_dir.path().file_name()?.to_str()?;
        let skill_md = skill_dir
            .files()
            .find(|f| f.path().file_name().and_then(|n| n.to_str()) == Some("SKILL.md"))?;
        let body = skill_md
            .contents_utf8()
            .expect("stock skill is valid UTF-8");
        Some((name, body))
    }))
}

/// Iterate stock template bodies as `(file_name, body)`.
#[cfg(test)]
pub fn templates() -> impl Iterator<Item = (&'static str, &'static str)> {
    let Some(dir) = CONTENT.get_dir("templates") else {
        return Box::new(std::iter::empty())
            as Box<dyn Iterator<Item = (&'static str, &'static str)>>;
    };
    Box::new(dir.files().filter_map(|f| {
        let name = f.path().file_name()?.to_str()?;
        if !name.ends_with(".md") {
            return None;
        }
        let body = f.contents_utf8().expect("stock content is valid UTF-8");
        Some((name, body))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_template_and_schema_exist() {
        assert!(template("explore").is_some_and(|t| t.contains("# explore")));
        assert!(schema("proposal").is_some_and(|s| s.contains("# Proposal schema")));
    }

    #[test]
    fn unknown_names_are_none() {
        assert!(template("not-a-real-template").is_none());
        assert!(schema("not-a-real-schema").is_none());
    }

    #[test]
    fn claude_commands_are_present() {
        assert!(has_harness("claude"));
        let names: Vec<_> = command_files("claude").map(|(n, _)| n).collect();
        assert!(names.contains(&"ds-explore.md"));
        assert!(!names.is_empty());
    }

    #[test]
    fn codex_skills_are_present() {
        assert!(has_harness("codex"));
        let names: Vec<_> = skill_dirs("codex").map(|(n, _)| n).collect();
        assert!(names.contains(&"ds-propose"));
        assert!(names.contains(&"ds-explore"));
        assert!(!names.is_empty());
        // Skill bodies carry frontmatter + stage instruction.
        let propose = skill_dirs("codex")
            .find(|(n, _)| *n == "ds-propose")
            .map(|(_, b)| b)
            .unwrap();
        assert!(propose.contains("name: ds-propose"));
        assert!(propose.contains("ds template propose"));
    }
}
