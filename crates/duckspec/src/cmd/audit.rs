use std::path::Path;

use duckpond::audit::{self, AuditReport, AuditScope};
use duckpond::config::Config;
use duckpond::error::ParseError;
use miette::NamedSource;
use owo_colors::OwoColorize;

use super::common::{find_duckspec_root, resolve_path};

pub fn run(change: Option<String>) -> anyhow::Result<()> {
    let duckspec_root = find_duckspec_root()?;
    let project_root = duckspec_root
        .parent()
        .ok_or_else(|| anyhow::anyhow!("duckspec/ has no parent directory"))?
        .to_path_buf();
    let config = Config::load(&duckspec_root).map_err(|e| anyhow::anyhow!("{e}"))?;

    let scope = match change {
        None => {
            eprintln!("  {} checking artifacts…", "·".dimmed());
            eprintln!("  {} scanning for backlinks…", "·".dimmed());
            AuditScope::Full
        }
        Some(input) => {
            let change_name = resolve_change_name(&input, &duckspec_root)?;
            eprintln!("  {} checking change {change_name}…", "·".dimmed());
            eprintln!("  {} scanning for backlinks…", "·".dimmed());
            AuditScope::Change(change_name)
        }
    };

    let report = audit::run_audit(&duckspec_root, &project_root, &config, scope)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let error_count = report.total_errors();
    print_report(&report, &project_root);

    if error_count > 0 {
        eprintln!();
        eprintln!("{}", format!("{error_count} audit error(s)").red().bold());
        std::process::exit(1);
    }

    eprintln!("  {} audit ok", "✓".green());
    Ok(())
}

/// Resolve a user-supplied change identifier (path or plain name) to a
/// change name.
fn resolve_change_name(input: &str, duckspec_root: &Path) -> anyhow::Result<String> {
    let change_dir = resolve_path(input, duckspec_root).or_else(|_| {
        let under_changes = duckspec_root.join("changes").join(input);
        if under_changes.is_dir() {
            Ok(under_changes)
        } else {
            anyhow::bail!("change not found: {input}")
        }
    })?;

    let changes_dir = duckspec_root.join("changes").canonicalize()?;
    let change_canonical = change_dir.canonicalize()?;
    let name = change_canonical
        .strip_prefix(&changes_dir)
        .map_err(|_| anyhow::anyhow!("{} is not under changes/", change_dir.display()))?
        .components()
        .next()
        .and_then(|c| c.as_os_str().to_str())
        .ok_or_else(|| anyhow::anyhow!("could not extract change name"))?
        .to_string();
    Ok(name)
}

// ---------------------------------------------------------------------------
// Report printing
// ---------------------------------------------------------------------------

fn print_report(report: &AuditReport, project_root: &Path) {
    // Per-file artifact errors (project-level).
    for group in &report.artifact_errors {
        let named = NamedSource::new(
            group.relative_path.display().to_string(),
            group.source.clone(),
        );
        report_parse_errors(&group.errors, named);
    }

    // Per-change file and cross-file errors.
    for group in &report.change_errors {
        for (path, source, errors) in &group.file_errors {
            let named = NamedSource::new(path.display().to_string(), source.clone());
            report_parse_errors(errors, named);
        }
        for err in &group.change_errors {
            eprintln!("  {} {err}", "×".red());
        }
    }

    for bl in &report.unresolved_backlinks {
        let display = bl
            .source_file
            .strip_prefix(project_root)
            .unwrap_or(&bl.source_file);
        eprintln!(
            "  {} {}:{} — backlink does not resolve: {}",
            "×".red(),
            display.display(),
            bl.line,
            bl.key.display()
        );
    }

    for key in &report.missing_backlink_scenarios {
        eprintln!(
            "  {} scenario marked test:code has no backlink: {}",
            "×".red(),
            key.display()
        );
    }

    for coverage in &report.missing_step_coverage {
        for key in &coverage.missing {
            eprintln!(
                "  {} test:code scenario not covered by step task: {}",
                "×".red(),
                key.display()
            );
        }
    }

    for r in &report.unresolved_step_refs {
        eprintln!(
            "  {} {}:{} — step @spec task does not resolve: {}",
            "×".red(),
            r.step_file.display(),
            r.line,
            r.key.display()
        );
    }
}

fn report_parse_errors(errors: &[ParseError], source: NamedSource<String>) {
    for err in errors {
        let report = miette::Report::new(err.clone()).with_source_code(source.clone());
        eprintln!("{report:?}");
    }
}
