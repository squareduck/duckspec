use std::path::{Path, PathBuf};

use duckpond::check::{self, CheckContext, DuckspecState, LoadedFile};
use duckpond::layout::{self, ArtifactKind};
use duckpond::merge;
use owo_colors::OwoColorize;

use super::common::{collect_files, find_duckspec_root, resolve_path};

pub fn run(name: String, dry: bool) -> anyhow::Result<()> {
    let duckspec_root = find_duckspec_root()?;
    let canonical_root = duckspec_root.canonicalize()?;

    // Resolve the change directory: try as a path first, fall back to
    // treating input as a bare change name under changes/.
    let change_dir = resolve_path(&name, &duckspec_root)
        .or_else(|_| {
            let under_changes = duckspec_root.join("changes").join(&name);
            if under_changes.is_dir() {
                Ok(under_changes)
            } else {
                anyhow::bail!("change not found: {name}")
            }
        })?;

    if !change_dir.is_dir() {
        anyhow::bail!("not a directory: {}", change_dir.display());
    }

    // Extract the change name from the resolved path.
    let change_canonical = change_dir.canonicalize()?;
    let changes_dir = duckspec_root.join("changes").canonicalize()?;
    let change_relative = change_canonical
        .strip_prefix(&changes_dir)
        .map_err(|_| {
            anyhow::anyhow!(
                "{} is not under changes/",
                change_dir.display()
            )
        })?;
    let name = change_relative
        .components()
        .next()
        .and_then(|c| c.as_os_str().to_str())
        .ok_or_else(|| anyhow::anyhow!("could not extract change name from path"))?
        .to_string();

    // Step 1: Validate the change.
    eprintln!("  {} validating change {name}…", "·".dimmed());
    validate_change(&duckspec_root, &name)?;

    // Step 2: Compute the archive plan.
    let plan = build_plan(&duckspec_root, &canonical_root, &change_dir)?;

    if dry {
        print_plan(&plan);
        return Ok(());
    }

    // Step 3: Execute the plan — collect results in memory first.
    let results = execute_plan(&duckspec_root, &plan)?;

    // Step 4: Write results and move to archive.
    // We write to a temp archive dir first, then do the swap, so we can
    // roll back by deleting the temp dir if re-validation fails.
    let archive_dir = pick_archive_dir(&duckspec_root, &name)?;
    apply_results(&duckspec_root, &results)?;
    std::fs::rename(&change_dir, &archive_dir)
        .map_err(|e| anyhow::anyhow!("failed to move change to archive: {e}"))?;

    // Step 5: Re-validate top-level caps.
    eprintln!("  {} re-validating caps…", "·".dimmed());
    if let Err(err) = validate_caps(&duckspec_root, &canonical_root) {
        // Rollback: move archive back, restore original files.
        eprintln!("  {} validation failed after archive, rolling back", "×".red());
        std::fs::rename(&archive_dir, &change_dir).ok();
        rollback_results(&duckspec_root, &results);
        return Err(err);
    }

    let archive_name = archive_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?");
    eprintln!(
        "  {} archived {name} → archive/{archive_name}",
        "✓".green()
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Plan
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct ArchivePlan {
    /// Full cap files to copy: (source in change, dest relative to caps/).
    copies: Vec<CopyOp>,
    /// Deltas to apply: (delta source in change, target file under caps/).
    deltas: Vec<DeltaOp>,
}

#[derive(Debug)]
struct CopyOp {
    /// Absolute path of the file in the change.
    source: PathBuf,
    /// Path relative to caps/, e.g. "auth/two-factor/spec.md".
    cap_relative: PathBuf,
}

#[derive(Debug)]
struct DeltaOp {
    /// Absolute path of the delta file in the change.
    delta_path: PathBuf,
    /// Path relative to caps/, e.g. "auth/spec.md".
    target_relative: PathBuf,
}

fn build_plan(
    _duckspec_root: &Path,
    canonical_root: &Path,
    change_dir: &Path,
) -> anyhow::Result<ArchivePlan> {
    let files = collect_files(change_dir)?;
    let mut copies = Vec::new();
    let mut deltas = Vec::new();

    for file_path in &files {
        let canonical = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Some(relative) = canonical.strip_prefix(canonical_root).ok() else {
            continue;
        };
        let Some(kind) = layout::classify(relative) else {
            continue;
        };

        // Extract the cap-relative path from within the change.
        // e.g. changes/add-two-factor/caps/auth/two-factor/spec.md
        //   → relative = changes/add-two-factor/caps/auth/two-factor/spec.md
        //   → we need auth/two-factor/spec.md
        let components: Vec<&str> = relative
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        match kind {
            ArtifactKind::ChangeCapSpec | ArtifactKind::ChangeCapDoc => {
                // changes/<name>/caps/<cap-path>/spec.md or doc.md
                // Skip "changes", "<name>", "caps" → rest is the cap-relative path.
                if components.len() > 3 && components[2] == "caps" {
                    let cap_relative: PathBuf = components[3..].iter().collect();
                    copies.push(CopyOp {
                        source: file_path.clone(),
                        cap_relative,
                    });
                }
            }
            ArtifactKind::SpecDelta => {
                if components.len() > 3 && components[2] == "caps" {
                    // e.g. auth/spec.delta.md → target is auth/spec.md
                    let cap_path: PathBuf = components[3..components.len() - 1].iter().collect();
                    deltas.push(DeltaOp {
                        delta_path: file_path.clone(),
                        target_relative: cap_path.join("spec.md"),
                    });
                }
            }
            ArtifactKind::DocDelta => {
                if components.len() > 3 && components[2] == "caps" {
                    let cap_path: PathBuf = components[3..components.len() - 1].iter().collect();
                    deltas.push(DeltaOp {
                        delta_path: file_path.clone(),
                        target_relative: cap_path.join("doc.md"),
                    });
                }
            }
            _ => {}
        }
    }

    copies.sort_by(|a, b| a.cap_relative.cmp(&b.cap_relative));
    deltas.sort_by(|a, b| a.target_relative.cmp(&b.target_relative));

    Ok(ArchivePlan { copies, deltas })
}

// ---------------------------------------------------------------------------
// Dry-run preview
// ---------------------------------------------------------------------------

fn print_plan(plan: &ArchivePlan) {
    if plan.copies.is_empty() && plan.deltas.is_empty() {
        eprintln!("  {} no capability changes to apply", "·".dimmed());
        return;
    }

    for op in &plan.copies {
        eprintln!(
            "  {} caps/{}",
            "add".green().bold(),
            op.cap_relative.display()
        );
    }

    for op in &plan.deltas {
        eprintln!(
            "  {} caps/{} ← {}",
            "merge".yellow().bold(),
            op.target_relative.display(),
            op.delta_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
        );
    }
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

struct ArchiveResult {
    /// Files to write: (path relative to caps/, content).
    writes: Vec<(PathBuf, String)>,
    /// Original contents for rollback: (path relative to caps/, Option<content>).
    /// None means the file didn't exist before (should be deleted on rollback).
    originals: Vec<(PathBuf, Option<String>)>,
}

fn execute_plan(duckspec_root: &Path, plan: &ArchivePlan) -> anyhow::Result<ArchiveResult> {
    let caps_dir = duckspec_root.join("caps");
    let mut writes = Vec::new();
    let mut originals = Vec::new();

    // Process copies.
    for op in &plan.copies {
        let dest = caps_dir.join(&op.cap_relative);
        let original = if dest.is_file() {
            Some(std::fs::read_to_string(&dest)?)
        } else {
            None
        };
        originals.push((op.cap_relative.clone(), original));

        let content = std::fs::read_to_string(&op.source)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", op.source.display()))?;
        writes.push((op.cap_relative.clone(), content));
    }

    // Process deltas.
    for op in &plan.deltas {
        let target_path = caps_dir.join(&op.target_relative);
        if !target_path.is_file() {
            anyhow::bail!(
                "delta target does not exist: caps/{}",
                op.target_relative.display()
            );
        }

        let source = std::fs::read_to_string(&target_path)?;
        let delta = std::fs::read_to_string(&op.delta_path)?;

        originals.push((op.target_relative.clone(), Some(source.clone())));

        let merged = merge::apply_delta(&source, &delta).map_err(|errs| {
            anyhow::anyhow!(
                "merge failed for caps/{}: {}",
                op.target_relative.display(),
                errs.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })?;

        match merged {
            Some(content) => writes.push((op.target_relative.clone(), content)),
            None => {
                // Delta deletes the file — record it but write nothing.
                // For now, we don't support file deletion via archive.
                anyhow::bail!(
                    "delta would delete caps/{} — not supported via archive",
                    op.target_relative.display()
                );
            }
        }
    }

    Ok(ArchiveResult { writes, originals })
}

fn apply_results(duckspec_root: &Path, results: &ArchiveResult) -> anyhow::Result<()> {
    let caps_dir = duckspec_root.join("caps");
    for (rel, content) in &results.writes {
        let dest = caps_dir.join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, content)
            .map_err(|e| anyhow::anyhow!("failed to write caps/{}: {e}", rel.display()))?;
    }
    Ok(())
}

fn rollback_results(duckspec_root: &Path, results: &ArchiveResult) {
    let caps_dir = duckspec_root.join("caps");
    for (rel, original) in &results.originals {
        let dest = caps_dir.join(rel);
        match original {
            Some(content) => {
                let _ = std::fs::write(&dest, content);
            }
            None => {
                let _ = std::fs::remove_file(&dest);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Archive directory naming
// ---------------------------------------------------------------------------

fn pick_archive_dir(duckspec_root: &Path, name: &str) -> anyhow::Result<PathBuf> {
    let now = time::OffsetDateTime::now_utc();
    let today = format!(
        "{:04}-{:02}-{:02}",
        now.year(),
        now.month() as u8,
        now.day()
    );
    let archive_root = duckspec_root.join("archive");
    std::fs::create_dir_all(&archive_root)?;

    // Find the next NN for today.
    let mut nn = 1u32;
    let prefix = format!("{today}-");
    if archive_root.is_dir() {
        for entry in std::fs::read_dir(&archive_root)? {
            let entry = entry?;
            if let Some(dir_name) = entry.file_name().to_str()
                && dir_name.starts_with(&prefix) {
                    // Try to extract NN from YYYY-MM-DD-NN-<name>.
                    let rest = &dir_name[prefix.len()..];
                    if let Some(dash_pos) = rest.find('-')
                        && let Ok(n) = rest[..dash_pos].parse::<u32>() {
                            nn = nn.max(n + 1);
                        }
                }
        }
    }

    Ok(archive_root.join(format!("{today}-{nn:02}-{name}")))
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_change(duckspec_root: &Path, name: &str) -> anyhow::Result<()> {
    let change_dir = duckspec_root.join("changes").join(name);
    let canonical_root = duckspec_root.canonicalize()?;
    let file_paths = collect_files(&change_dir)?;
    let mut loaded_files = Vec::new();

    for file_path in &file_paths {
        let canonical = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Some(relative) = canonical.strip_prefix(&canonical_root).ok() else {
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

    // Build top-level state.
    let state = build_duckspec_state(duckspec_root)?;
    let result = check::check_change(name, &loaded_files, &state);

    let mut error_count = 0usize;
    for (path, errors) in &result.file_errors {
        for err in errors {
            eprintln!("  {} {}: {err}", "×".red(), path.display());
        }
        error_count += errors.len();
    }
    for err in &result.change_errors {
        eprintln!("  {} {err}", "×".red());
        error_count += 1;
    }

    if error_count > 0 {
        anyhow::bail!(
            "change {name} has {error_count} validation error(s) — fix before archiving"
        );
    }

    Ok(())
}

fn validate_caps(duckspec_root: &Path, canonical_root: &Path) -> anyhow::Result<()> {
    let caps_dir = duckspec_root.join("caps");
    if !caps_dir.is_dir() {
        return Ok(());
    }

    let files = collect_files(&caps_dir)?;
    let mut error_count = 0usize;

    for file_path in &files {
        let canonical = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Some(relative) = canonical.strip_prefix(canonical_root).ok() else {
            continue;
        };
        let Some(kind) = layout::classify(relative) else {
            continue;
        };

        let source = std::fs::read_to_string(file_path)?;
        let ctx = build_context(&kind, relative);
        let result = check::check_artifact(&source, &kind, &ctx);

        for err in &result.errors {
            eprintln!("  {} {}: {err}", "×".red(), relative.display());
        }
        error_count += result.errors.len();
    }

    if error_count > 0 {
        anyhow::bail!("caps have {error_count} validation error(s) after archive");
    }

    Ok(())
}

fn build_context(kind: &ArtifactKind, relative: &Path) -> CheckContext {
    if *kind == ArtifactKind::Step {
        let filename = relative
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("");
        CheckContext {
            filename_slug: layout::extract_step_slug(filename),
        }
    } else {
        CheckContext::default()
    }
}

fn build_duckspec_state(duckspec_root: &Path) -> anyhow::Result<DuckspecState> {
    use std::collections::HashSet;
    let caps_dir = duckspec_root.join("caps");
    let mut cap_spec_paths = HashSet::new();
    let mut cap_doc_paths = HashSet::new();

    if caps_dir.is_dir() {
        scan_caps(&caps_dir, &caps_dir, &mut cap_spec_paths, &mut cap_doc_paths)?;
    }

    Ok(DuckspecState {
        cap_spec_paths,
        cap_doc_paths,
    })
}

fn scan_caps(
    dir: &Path,
    caps_root: &Path,
    spec_paths: &mut std::collections::HashSet<PathBuf>,
    doc_paths: &mut std::collections::HashSet<PathBuf>,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
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
                    "spec.md" => { spec_paths.insert(cap_path); }
                    "doc.md" => { doc_paths.insert(cap_path); }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
