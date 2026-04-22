use std::collections::HashSet;
use std::path::{Path, PathBuf};

use duckpond::check::{self, CheckContext, DuckspecState, LoadedFile};
use duckpond::error::ParseError;
use duckpond::layout::{self, ArtifactKind};
use miette::NamedSource;
use owo_colors::OwoColorize;

use super::common::{collect_files, find_duckspec_root, resolve_path};

pub fn run(path: Option<String>) -> anyhow::Result<()> {
    let duckspec_root = find_duckspec_root()?;
    let canonical_root = duckspec_root.canonicalize()?;

    // Determine whether this is a change-level check.
    let resolved = resolve_target(&duckspec_root, path.as_deref())?;

    match resolved {
        Target::ChangeDir { change_name } => run_change_check(&duckspec_root, &change_name),
        Target::FileOrDir { path: scan_path } => {
            run_file_check(&duckspec_root, &canonical_root, &scan_path)
        }
    }
}

enum Target {
    /// Path is `changes/<name>` — run change-level checks.
    ChangeDir { change_name: String },
    /// Path is a file, a non-change directory, or the whole tree.
    FileOrDir { path: PathBuf },
}

/// Resolve the user-provided path and detect change-level mode.
fn resolve_target(duckspec_root: &Path, target: Option<&str>) -> anyhow::Result<Target> {
    let scan_path = match target {
        Some(t) => resolve_path(t, duckspec_root)?,
        None => {
            return Ok(Target::FileOrDir {
                path: duckspec_root.to_path_buf(),
            });
        }
    };

    // Check if this is a change directory.
    if scan_path.is_dir() {
        let canonical_scan = scan_path.canonicalize()?;
        let canonical_root = duckspec_root.canonicalize()?;
        if let Ok(relative) = canonical_scan.strip_prefix(&canonical_root) {
            let components: Vec<&str> = relative
                .components()
                .filter_map(|c| c.as_os_str().to_str())
                .collect();
            if components.len() == 2 && components[0] == "changes" {
                return Ok(Target::ChangeDir {
                    change_name: components[1].to_string(),
                });
            }
        }
    }

    Ok(Target::FileOrDir { path: scan_path })
}

/// Run single-file checks on a file or directory.
fn run_file_check(
    _duckspec_root: &Path,
    canonical_root: &Path,
    scan_path: &Path,
) -> anyhow::Result<()> {
    let files = collect_files(scan_path)?;
    let mut error_count: usize = 0;
    let mut has_order_violation = false;

    for file_path in &files {
        let canonical_file = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Some(relative) = canonical_file.strip_prefix(canonical_root).ok() else {
            eprintln!("skipping {}: not under duckspec/", file_path.display());
            continue;
        };

        let Some(kind) = layout::classify(relative) else {
            continue;
        };

        let source = std::fs::read_to_string(file_path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", file_path.display()))?;

        let ctx = build_context(&kind, relative);
        let result = check::check_artifact(&source, &kind, &ctx);

        if !result.errors.is_empty() {
            if result
                .errors
                .iter()
                .any(|e| matches!(e, ParseError::DeltaOrderViolation { .. }))
            {
                has_order_violation = true;
            }
            let named = NamedSource::new(relative.display().to_string(), source.clone());
            report_parse_errors(&result.errors, named);
            error_count += result.errors.len();
        }
    }

    if error_count > 0 {
        report_summary(error_count);
        if has_order_violation {
            eprintln!(
                "  {} run `ds format` to fix delta ordering",
                "hint:".yellow()
            );
        }
        std::process::exit(1);
    }

    if !files.is_empty() {
        report_ok();
    }

    Ok(())
}

/// Run change-level checks on a change directory.
fn run_change_check(duckspec_root: &Path, change_name: &str) -> anyhow::Result<()> {
    let change_dir = duckspec_root.join("changes").join(change_name);
    let canonical_root = duckspec_root.canonicalize()?;

    // Load all files in the change.
    let file_paths = collect_files(&change_dir)?;
    let mut loaded_files = Vec::new();

    for file_path in &file_paths {
        let canonical_file = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Some(relative) = canonical_file.strip_prefix(&canonical_root).ok() else {
            continue;
        };
        let Some(kind) = layout::classify(relative) else {
            continue;
        };
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", file_path.display()))?;

        loaded_files.push(LoadedFile {
            relative_path: relative.to_path_buf(),
            kind,
            content,
        });
    }

    // Build top-level state by scanning caps/.
    let state = build_duckspec_state(duckspec_root)?;

    let result = check::check_change(change_name, &loaded_files, &state);

    let mut error_count: usize = 0;

    // Report per-file errors with source context.
    // Build a map from relative path to content for source lookups.
    let source_map: std::collections::HashMap<&Path, &str> = loaded_files
        .iter()
        .map(|f| (f.relative_path.as_path(), f.content.as_str()))
        .collect();

    for (path, errors) in &result.file_errors {
        if let Some(&source) = source_map.get(path.as_path()) {
            let named = NamedSource::new(path.display().to_string(), source.to_string());
            report_parse_errors(errors, named);
        } else {
            for err in errors {
                eprintln!("{}  error: {err}", path.display());
            }
        }
        error_count += errors.len();
    }

    for err in &result.change_errors {
        eprintln!("  {} {err}", "×".red());
        error_count += 1;
    }

    if error_count > 0 {
        report_summary(error_count);
        std::process::exit(1);
    }

    report_ok();
    Ok(())
}

/// Build `CheckContext` for a file based on its kind and path.
fn build_context(kind: &ArtifactKind, relative: &Path) -> CheckContext {
    if *kind == ArtifactKind::Step {
        let filename = relative.file_name().and_then(|f| f.to_str()).unwrap_or("");
        CheckContext {
            filename_slug: layout::extract_step_slug(filename),
        }
    } else {
        CheckContext::default()
    }
}

/// Scan `duckspec/caps/` for existing spec.md and doc.md files.
fn build_duckspec_state(duckspec_root: &Path) -> anyhow::Result<DuckspecState> {
    let caps_dir = duckspec_root.join("caps");
    let mut cap_spec_paths = HashSet::new();
    let mut cap_doc_paths = HashSet::new();

    if caps_dir.is_dir() {
        scan_caps(
            &caps_dir,
            &caps_dir,
            &mut cap_spec_paths,
            &mut cap_doc_paths,
        )?;
    }

    Ok(DuckspecState {
        cap_spec_paths,
        cap_doc_paths,
    })
}

/// Recursively scan `caps/` for spec.md and doc.md files.
fn scan_caps(
    dir: &Path,
    caps_root: &Path,
    spec_paths: &mut HashSet<PathBuf>,
    doc_paths: &mut HashSet<PathBuf>,
) -> anyhow::Result<()> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            scan_caps(&path, caps_root, spec_paths, doc_paths)?;
        } else if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            let cap_path = path
                .parent()
                .and_then(|p| p.strip_prefix(caps_root).ok())
                .map(|p| p.to_path_buf());
            if let Some(cap_path) = cap_path {
                match filename {
                    "spec.md" => {
                        spec_paths.insert(cap_path);
                    }
                    "doc.md" => {
                        doc_paths.insert(cap_path);
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

/// Print parse errors with miette source-annotated output.
fn report_parse_errors(errors: &[ParseError], source: NamedSource<String>) {
    for err in errors {
        let report = miette::Report::new(err.clone()).with_source_code(source.clone());
        eprintln!("{report:?}");
    }
}

fn report_summary(error_count: usize) {
    eprintln!();
    eprintln!("{}", format!("{error_count} error(s) found").red().bold());
}

fn report_ok() {
    eprintln!("  {} ok", "✓".green());
}
