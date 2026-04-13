use std::fs;
use std::path::Path;

use super::common::find_duckspec_root;

/// Embedded template directory, included at compile time via `include_str!`.
/// At runtime we read from the content directory relative to the binary,
/// but for now we read from the duckspec crate's content directory.
const TEMPLATE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/content/templates");

pub fn run(name: String) -> anyhow::Result<()> {
    let template_path = format!("{TEMPLATE_DIR}/{name}.md");
    let template = fs::read_to_string(&template_path)
        .map_err(|_| anyhow::anyhow!("unknown template: {name}"))?;

    // Look for hook files in duckspec/hooks/.
    let duckspec_root = find_duckspec_root().ok();
    let pre_hook = duckspec_root
        .as_ref()
        .and_then(|root| read_hook_content(root, &name, "pre"));
    let post_hook = duckspec_root
        .as_ref()
        .and_then(|root| read_hook_content(root, &name, "post"));

    let output = apply_hooks(&template, pre_hook.as_deref(), post_hook.as_deref());
    print!("{output}");

    Ok(())
}

/// Read a hook file and return everything after the H1 line.
fn read_hook_content(duckspec_root: &Path, stage: &str, position: &str) -> Option<String> {
    let path = duckspec_root.join(format!("hooks/{stage}-{position}.md"));
    let content = fs::read_to_string(path).ok()?;

    // Skip the H1 line, return the rest.
    let after_h1 = content
        .lines()
        .skip_while(|line| !line.starts_with("# "))
        .skip(1) // skip the H1 itself
        .collect::<Vec<_>>()
        .join("\n");

    let trimmed = after_h1.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Replace or remove `## Hook - Pre` and `## Hook - Post` sections
/// in the template.
fn apply_hooks(template: &str, pre: Option<&str>, post: Option<&str>) -> String {
    let mut output = String::new();
    let mut lines = template.lines().peekable();

    while let Some(line) = lines.next() {
        if line.trim() == "## Hook - Pre" {
            // Skip until the next heading or EOF.
            skip_section(&mut lines);
            if let Some(content) = pre {
                output.push_str(content);
                output.push('\n');
                output.push('\n');
            }
        } else if line.trim() == "## Hook - Post" {
            skip_section(&mut lines);
            if let Some(content) = post {
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

## Hook - Pre

## Instructions

Do stuff.

## Hook - Post
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
    fn hooks_replaced_when_present() {
        let template = "\
# Template

## Hook - Pre

## Instructions

Do stuff.

## Hook - Post
";
        let result = apply_hooks(template, Some("Pre content here."), Some("Post content here."));
        assert_eq!(
            result,
            "\
# Template

Pre content here.

## Instructions

Do stuff.

Post content here.
"
        );
    }
}
