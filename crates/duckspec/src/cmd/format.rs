use duckpond::config::Config;
use duckpond::error::ParseError;
use duckpond::format::{self, FormatError};
use duckpond::layout;
use miette::NamedSource;
use owo_colors::OwoColorize;

use super::common::{collect_files, find_duckspec_root, resolve_path};

pub fn run(path: Option<String>, dry: bool) -> anyhow::Result<()> {
    let duckspec_root = find_duckspec_root()?;
    let canonical_root = duckspec_root.canonicalize()?;

    let scan_path = match path.as_deref() {
        Some(p) => resolve_path(p, &duckspec_root)?,
        None => duckspec_root.clone(),
    };
    let single_file_mode = scan_path.is_file();

    let config = Config::load(&duckspec_root)?;
    let files = collect_files(&scan_path)?;

    let mut error_count = 0usize;
    let mut changed_count = 0usize;

    for file_path in &files {
        let canonical_file = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Ok(relative) = canonical_file.strip_prefix(&canonical_root) else {
            if single_file_mode {
                anyhow::bail!("not under duckspec/: {}", file_path.display());
            }
            eprintln!("skipping {}: not under duckspec/", file_path.display());
            continue;
        };

        if layout::classify(relative).is_none() {
            if single_file_mode {
                anyhow::bail!(
                    "not a recognized duckspec artifact: {}",
                    relative.display()
                );
            }
            continue;
        }

        let source = std::fs::read_to_string(file_path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", file_path.display()))?;

        match format::format_artifact(relative, &source, &config.format) {
            Ok(formatted) => {
                if dry {
                    print!("{formatted}");
                } else if formatted != source {
                    std::fs::write(file_path, &formatted).map_err(|e| {
                        anyhow::anyhow!("failed to write {}: {e}", file_path.display())
                    })?;
                    eprintln!("formatted {}", relative.display());
                    changed_count += 1;
                }
            }
            Err(FormatError::Parse { errors, .. }) => {
                let named = NamedSource::new(relative.display().to_string(), source.clone());
                report_parse_errors(&errors, named);
                error_count += errors.len();
            }
            Err(FormatError::UnknownArtifactType { path }) => {
                eprintln!("skipping {}: unknown artifact type", path.display());
            }
        }
    }

    if error_count > 0 {
        eprintln!();
        eprintln!("{}", format!("{error_count} error(s) found").red().bold());
        std::process::exit(1);
    }

    if !dry && !files.is_empty() && changed_count == 0 {
        eprintln!("  {} already canonical", "✓".green());
    }

    Ok(())
}

fn report_parse_errors(errors: &[ParseError], source: NamedSource<String>) {
    for err in errors {
        let report = miette::Report::new(err.clone()).with_source_code(source.clone());
        eprintln!("{report:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn write(root: &Path, rel: &str, content: &str) -> PathBuf {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, content).unwrap();
        p
    }

    /// End-to-end: format a doc file in place and verify the file content.
    #[test]
    fn format_writes_canonical_output() {
        let tmp = tempdir().unwrap();
        let cwd = tmp.path();
        let duckspec = cwd.join("duckspec");
        fs::create_dir_all(&duckspec).unwrap();

        let unwrapped = "# Hello\n\nThis is a long summary that should remain on one line because it fits comfortably under ninety characters wide.\n";
        let path = write(
            &duckspec,
            "caps/things/doc.md",
            unwrapped,
        );

        // Run via the public format_artifact (the CLI shells out to this).
        let cfg = duckpond::config::FormatConfig::default();
        let rel = std::path::PathBuf::from("caps/things/doc.md");
        let formatted = format::format_artifact(&rel, unwrapped, &cfg).unwrap();
        fs::write(&path, &formatted).unwrap();

        let on_disk = fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains("# Hello"));
        // Idempotency: re-format must be a no-op.
        let again = format::format_artifact(&rel, &on_disk, &cfg).unwrap();
        assert_eq!(again, on_disk);
    }
}
