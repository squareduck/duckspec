use std::fs;

use duckpond::config::Config;

use super::common::find_duckspec_root;

const SCHEMA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/content/schemas");

pub fn run(name: String) -> anyhow::Result<()> {
    let schema_path = format!("{SCHEMA_DIR}/{name}.md");
    let content = fs::read_to_string(&schema_path)
        .map_err(|_| anyhow::anyhow!("unknown schema: {name}"))?;

    let line_width = find_duckspec_root()
        .ok()
        .and_then(|root| Config::load(&root).ok())
        .unwrap_or_default()
        .format
        .line_width;

    let output = inject_formatting(&content, line_width);
    print!("{output}");
    Ok(())
}

/// Insert a `## Formatting` section into a schema, rendering the configured
/// line width. Placed just before `## Example` when present, else appended.
fn inject_formatting(content: &str, line_width: usize) -> String {
    let section = format!(
        "## Formatting\n\
         \n\
         - Wrap prose lines at {line_width} characters. Break at word boundaries.\n\
         - Do not wrap inside fenced code blocks, tables, URLs, or long\n  \
         inline identifiers where a break would distort meaning.\n\
         - Blank lines separate blocks; keep lists single-spaced unless\n  \
         items contain paragraphs.\n\
         \n"
    );

    match content.find("\n## Example") {
        Some(idx) => {
            let (before, after) = content.split_at(idx + 1);
            format!("{before}{section}{after}")
        }
        None => {
            let trimmed = content.trim_end();
            format!("{trimmed}\n\n{section}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_before_example() {
        let input =
            "# Schema\n\nIntro.\n\n## Rules\n\nRule text.\n\n## Example\n\nExample text.\n";
        let out = inject_formatting(input, 80);
        let rules = out.find("## Rules").unwrap();
        let fmt = out.find("## Formatting").unwrap();
        let ex = out.find("## Example").unwrap();
        assert!(rules < fmt && fmt < ex);
        assert!(out.contains("Wrap prose lines at 80 characters"));
        assert!(out.contains("Example text."));
    }

    #[test]
    fn appends_when_no_example() {
        let input = "# Schema\n\n## Rules\n\nRule text.\n";
        let out = inject_formatting(input, 100);
        assert!(out.contains("## Formatting"));
        assert!(out.contains("Wrap prose lines at 100 characters"));
    }

    #[test]
    fn substitutes_line_width() {
        let input = "# Schema\n\n## Example\n";
        let out = inject_formatting(input, 120);
        assert!(out.contains("Wrap prose lines at 120 characters"));
    }

    #[test]
    fn preserves_content_before_and_after_example() {
        let input = "# A\n\nB\n\n## Example\n\nC\n";
        let out = inject_formatting(input, 80);
        assert!(out.starts_with("# A\n\nB\n\n## Formatting"));
        assert!(out.ends_with("## Example\n\nC\n"));
    }
}
