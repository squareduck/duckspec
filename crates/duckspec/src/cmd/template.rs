use std::fs;
use std::path::Path;

use super::common::find_duckspec_root;
use crate::content;

pub fn run(name: String) -> anyhow::Result<()> {
    let template =
        content::template(&name).ok_or_else(|| anyhow::anyhow!("unknown template: {name}"))?;

    let duckspec_root = find_duckspec_root().ok();
    let before = duckspec_root
        .as_ref()
        .and_then(|root| read_hook_content(root, &name, "before"));
    let after = duckspec_root
        .as_ref()
        .and_then(|root| read_hook_content(root, &name, "after"));

    let output = apply_hooks(template, before.as_deref(), after.as_deref());
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

    const TEMPLATE_WITH_HOOK_PLACEHOLDERS: &str = "\
# Template

## Before write

## Instructions

Do stuff.

## After write
";

    // @spec cli/hooks Template injection: Non-empty hooks inject under section headers
    #[test]
    fn non_empty_hooks_inject_under_section_headers() {
        let result = apply_hooks(
            TEMPLATE_WITH_HOOK_PLACEHOLDERS,
            Some("Before content here."),
            Some("After content here."),
        );
        assert_eq!(
            result,
            "\
# Template

## Before write

Before content here.

## Instructions

Do stuff.

## After write

After content here.
"
        );
    }

    // @spec cli/hooks Template injection: Missing or empty hooks drop the placeholders
    #[test]
    fn missing_or_empty_hooks_drop_the_placeholders() {
        let missing = apply_hooks(TEMPLATE_WITH_HOOK_PLACEHOLDERS, None, None);
        assert!(
            !missing.contains("## Before write"),
            "missing before hook must drop the section"
        );
        assert!(
            !missing.contains("## After write"),
            "missing after hook must drop the section"
        );
        assert_eq!(
            missing,
            "\
# Template

## Instructions

Do stuff.

"
        );

        let tmp = tempfile::tempdir().unwrap();
        let hooks_dir = tmp.path().join("hooks");
        fs::create_dir(&hooks_dir).unwrap();
        fs::write(hooks_dir.join("step-before.md"), "   \n\n  \t\n").unwrap();

        let empty_before = read_hook_content(tmp.path(), "step", "before");
        assert!(empty_before.is_none(), "whitespace-only file is absent");
        let empty = apply_hooks(TEMPLATE_WITH_HOOK_PLACEHOLDERS, empty_before.as_deref(), None);
        assert!(!empty.contains("## Before write"));
        assert!(!empty.contains("## After write"));
    }

    // @spec cli/hooks Template injection: Body without H1 is rendered verbatim
    #[test]
    fn body_without_h1_is_rendered_verbatim() {
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
        let mut count = 0;
        for (name, body) in content::templates() {
            count += 1;
            assert!(
                body.contains("## Before write"),
                "{name} is missing `## Before write` placeholder"
            );
            assert!(
                body.contains("## After write"),
                "{name} is missing `## After write` placeholder"
            );
        }
        assert!(count > 0, "expected at least one template");
    }
}
