use std::fs;
use std::path::Path;

use super::common::find_duckspec_root;

const TEMPLATE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/content/templates");

pub fn run(name: String) -> anyhow::Result<()> {
    let template_path = format!("{TEMPLATE_DIR}/{name}.md");
    let template = fs::read_to_string(&template_path)
        .map_err(|_| anyhow::anyhow!("unknown template: {name}"))?;

    let duckspec_root = find_duckspec_root().ok();
    let before = duckspec_root
        .as_ref()
        .and_then(|root| read_hook_content(root, &name, "before"));
    let after = duckspec_root
        .as_ref()
        .and_then(|root| read_hook_content(root, &name, "after"));

    let output = apply_hooks(&template, before.as_deref(), after.as_deref());
    print!("{output}");

    Ok(())
}

/// Read a hook file and return its contents (trimmed). Returns `None` if the
/// file is missing, unreadable, or contains only whitespace.
fn read_hook_content(duckspec_root: &Path, stage: &str, position: &str) -> Option<String> {
    let path = duckspec_root.join(format!("hooks/{stage}-{position}.md"));
    let content = fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Replace `## Before write` and `## After write` placeholders. When a hook
/// is present, emit the header followed by the hook body. When absent, drop
/// the placeholder line entirely.
fn apply_hooks(template: &str, before: Option<&str>, after: Option<&str>) -> String {
    let mut output = String::new();
    let mut lines = template.lines().peekable();

    while let Some(line) = lines.next() {
        if line.trim() == "## Before write" {
            skip_section(&mut lines);
            if let Some(content) = before {
                output.push_str("## Before write\n\n");
                output.push_str(content);
                output.push_str("\n\n");
            }
        } else if line.trim() == "## After write" {
            skip_section(&mut lines);
            if let Some(content) = after {
                output.push_str("## After write\n\n");
                output.push_str(content);
                output.push('\n');
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }

    output
}

/// Advance the iterator past the current section (until the next heading
/// of equal or higher level, or EOF).
fn skip_section(lines: &mut std::iter::Peekable<std::str::Lines<'_>>) {
    while let Some(next) = lines.peek() {
        if next.starts_with("## ") || next.starts_with("# ") {
            break;
        }
        lines.next();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hooks_removed_when_absent() {
        let template = "\
# Template

## Before write

## Instructions

Do stuff.

## After write
";
        let result = apply_hooks(template, None, None);
        assert_eq!(
            result,
            "\
# Template

## Instructions

Do stuff.

"
        );
    }

    #[test]
    fn hooks_inserted_with_headers_when_present() {
        let template = "\
# Template

## Before write

## Instructions

Do stuff.

## After write
";
        let result = apply_hooks(
            template,
            Some("Pre content here."),
            Some("Post content here."),
        );
        assert_eq!(
            result,
            "\
# Template

## Before write

Pre content here.

## Instructions

Do stuff.

## After write

Post content here.
"
        );
    }

    #[test]
    fn hook_without_h1_is_rendered_verbatim() {
        let template = "\
# Template

## Before write

## Body
";
        let result = apply_hooks(template, Some("Just text, no heading."), None);
        assert_eq!(
            result,
            "\
# Template

## Before write

Just text, no heading.

## Body
"
        );
    }

    #[test]
    fn empty_hook_file_treated_as_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let hooks_dir = tmp.path().join("hooks");
        fs::create_dir(&hooks_dir).unwrap();
        fs::write(hooks_dir.join("step-before.md"), "   \n\n  \t\n").unwrap();

        let result = read_hook_content(tmp.path(), "step", "before");
        assert!(result.is_none());
    }

    #[test]
    fn read_hook_content_returns_trimmed_body() {
        let tmp = tempfile::tempdir().unwrap();
        let hooks_dir = tmp.path().join("hooks");
        fs::create_dir(&hooks_dir).unwrap();
        fs::write(
            hooks_dir.join("step-before.md"),
            "\n\n  hello world  \n\n\n",
        )
        .unwrap();

        let result = read_hook_content(tmp.path(), "step", "before");
        assert_eq!(result.as_deref(), Some("hello world"));
    }

    #[test]
    fn every_stock_template_has_hook_placeholders() {
        let template_dir = Path::new(TEMPLATE_DIR);
        let entries = fs::read_dir(template_dir).expect("read templates dir");
        let mut count = 0;
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "md") {
                continue;
            }
            count += 1;
            let content = fs::read_to_string(&path).unwrap();
            let name = path.file_name().unwrap().to_string_lossy();
            assert!(
                content.contains("## Before write"),
                "{name} is missing `## Before write` placeholder"
            );
            assert!(
                content.contains("## After write"),
                "{name} is missing `## After write` placeholder"
            );
        }
        assert!(count > 0, "expected at least one template");
    }
}
