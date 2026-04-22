use std::path::Path;

use duckpond::layout::{self, ArtifactKind};
use duckpond::parse;
use owo_colors::OwoColorize;

use super::common::{collect_files, find_duckspec_root};

pub fn run(caps: bool, codex: bool, project: bool) -> anyhow::Result<()> {
    let duckspec_root = find_duckspec_root()?;
    let canonical_root = duckspec_root.canonicalize()?;

    // If no filter flags, show everything.
    let show_all = !caps && !codex && !project;

    if show_all || project {
        print_project(&duckspec_root, &canonical_root)?;
    }

    if show_all || caps {
        print_caps(&duckspec_root, &canonical_root)?;
    }

    if show_all || codex {
        print_codex(&duckspec_root, &canonical_root)?;
    }

    Ok(())
}

/// Parse a file and extract its title and summary.
fn extract_title_summary(source: &str, kind: &ArtifactKind) -> Option<(String, String)> {
    let elements = parse::parse_elements(source);
    match kind {
        ArtifactKind::CapSpec | ArtifactKind::ChangeCapSpec => {
            let spec = parse::spec::parse_spec(&elements).ok()?;
            Some((spec.title, spec.summary))
        }
        ArtifactKind::CapDoc
        | ArtifactKind::ChangeCapDoc
        | ArtifactKind::Proposal
        | ArtifactKind::Design
        | ArtifactKind::Codex
        | ArtifactKind::Project => {
            let doc = parse::doc::parse_document(&elements).ok()?;
            Some((doc.title, doc.summary))
        }
        ArtifactKind::Step => {
            let step = parse::step::parse_step(&elements).ok()?;
            Some((step.title, step.summary))
        }
        ArtifactKind::SpecDelta | ArtifactKind::DocDelta => {
            let delta = parse::delta::parse_delta(&elements).ok()?;
            Some((delta.title, delta.summary.unwrap_or_default()))
        }
    }
}

/// Extract spec stats: (requirement_count, scenario_count).
fn extract_spec_stats(source: &str) -> Option<(usize, usize)> {
    let elements = parse::parse_elements(source);
    let spec = parse::spec::parse_spec(&elements).ok()?;
    let scenarios: usize = spec.requirements.iter().map(|r| r.scenarios.len()).sum();
    Some((spec.requirements.len(), scenarios))
}

/// Print a summary string, indenting continuation lines to align with the first.
fn print_summary(summary: &str, indent: &str) {
    let mut lines = summary.lines();
    if let Some(first) = lines.next() {
        eprint!("{indent}{}", first.dimmed());
        for line in lines {
            eprint!("\n{indent}{}", line.dimmed());
        }
        eprintln!();
    }
}

fn print_project(duckspec_root: &Path, canonical_root: &Path) -> anyhow::Result<()> {
    let project_path = duckspec_root.join("project.md");
    if !project_path.is_file() {
        return Ok(());
    }

    let source = std::fs::read_to_string(&project_path)?;
    let relative = project_path
        .canonicalize()?
        .strip_prefix(canonical_root)
        .unwrap_or(Path::new("project.md"))
        .to_path_buf();
    let kind = layout::classify(&relative).unwrap_or(ArtifactKind::Project);

    if let Some((title, summary)) = extract_title_summary(&source, &kind) {
        eprintln!(
            "  {} {}",
            "project.md".bold(),
            format!("— {title}").dimmed()
        );
        if !summary.is_empty() {
            print_summary(&summary, "    ");
        }
        eprintln!();
    }

    Ok(())
}

fn print_caps(duckspec_root: &Path, canonical_root: &Path) -> anyhow::Result<()> {
    let caps_dir = duckspec_root.join("caps");
    if !caps_dir.is_dir() {
        return Ok(());
    }

    eprintln!("  {}", "caps/".bold());

    // Print child capability directories (not the caps/ dir itself).
    let mut entries: Vec<_> = std::fs::read_dir(&caps_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in &entries {
        print_cap_tree(&entry.path(), canonical_root, 2)?;
    }
    eprintln!();
    Ok(())
}

/// Recursively print capability tree.
fn print_cap_tree(dir: &Path, canonical_root: &Path, depth: usize) -> anyhow::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    // Collect info about this capability directory.
    let spec_path = dir.join("spec.md");
    let doc_path = dir.join("doc.md");
    let indent = "  ".repeat(depth);

    // Find the capability name from the directory.
    let cap_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Try to get title and summary from spec.
    let mut title = None;
    let mut summary = None;
    let mut stats_str = String::new();

    if spec_path.is_file() {
        let source = std::fs::read_to_string(&spec_path)?;
        let relative = spec_path
            .canonicalize()?
            .strip_prefix(canonical_root)
            .map(|p| p.to_path_buf())
            .unwrap_or_default();
        let kind = layout::classify(&relative);
        if let Some(kind) = kind
            && let Some((t, s)) = extract_title_summary(&source, &kind)
        {
            title = Some(t);
            if !s.is_empty() {
                summary = Some(s);
            }
        }
        if let Some((reqs, scenarios)) = extract_spec_stats(&source) {
            stats_str = format!("{reqs} req, {scenarios} scn");
        }
    }

    let has_doc = doc_path.is_file();

    // Print capability line.
    let artifacts: Vec<&str> = [
        if spec_path.is_file() {
            Some("spec")
        } else {
            None
        },
        if has_doc { Some("doc") } else { None },
    ]
    .into_iter()
    .flatten()
    .collect();

    let title_part = match &title {
        Some(t) => format!(" — {t}"),
        None => String::new(),
    };

    let meta_parts: Vec<String> = [
        if !artifacts.is_empty() {
            Some(artifacts.join(", "))
        } else {
            None
        },
        if !stats_str.is_empty() {
            Some(stats_str)
        } else {
            None
        },
    ]
    .into_iter()
    .flatten()
    .collect();

    let meta = if !meta_parts.is_empty() {
        format!(" ({})", meta_parts.join("; "))
    } else {
        String::new()
    };

    eprintln!(
        "{indent}{}{} {}",
        format!("{cap_name}/").bold(),
        title_part.dimmed(),
        meta.dimmed()
    );

    if let Some(ref s) = summary {
        let summary_indent = format!("{indent}  ");
        print_summary(s, &summary_indent);
    }

    // Recurse into subdirectories (child capabilities).
    for entry in &entries {
        let path = entry.path();
        if path.is_dir() {
            print_cap_tree(&path, canonical_root, depth + 1)?;
        }
    }

    Ok(())
}

fn print_codex(duckspec_root: &Path, canonical_root: &Path) -> anyhow::Result<()> {
    let codex_dir = duckspec_root.join("codex");
    if !codex_dir.is_dir() {
        return Ok(());
    }

    eprintln!("  {}", "codex/".bold());

    let files = collect_files(&codex_dir)?;
    for file_path in &files {
        let canonical_file = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Some(relative) = canonical_file.strip_prefix(canonical_root).ok() else {
            continue;
        };
        let Some(kind) = layout::classify(relative) else {
            continue;
        };

        let source = std::fs::read_to_string(file_path)?;
        let display_path = relative.strip_prefix("codex").unwrap_or(relative);

        if let Some((title, summary)) = extract_title_summary(&source, &kind) {
            eprintln!(
                "    {} {}",
                display_path.display().to_string().bold(),
                format!("— {title}").dimmed()
            );
            if !summary.is_empty() {
                print_summary(&summary, "      ");
            }
        }
    }
    eprintln!();

    Ok(())
}
