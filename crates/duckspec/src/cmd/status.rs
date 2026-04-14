use std::path::Path;

use duckpond::artifact::spec::TestMarkerKind;
use duckpond::layout::{self, ArtifactKind};
use duckpond::merge;
use duckpond::parse;
use owo_colors::OwoColorize;

use super::common::{collect_files, find_duckspec_root, list_subdirs, resolve_path};

pub fn run(path: Option<String>) -> anyhow::Result<()> {
    let duckspec_root = find_duckspec_root()?;
    let canonical_root = duckspec_root.canonicalize()?;

    match path {
        None => run_project_status(&duckspec_root, &canonical_root),
        Some(input) => run_path_status(&duckspec_root, &canonical_root, &input),
    }
}

// ===========================================================================
// Project-level status (no path)
// ===========================================================================

fn run_project_status(duckspec_root: &Path, canonical_root: &Path) -> anyhow::Result<()> {
    let cap_count = count_caps(duckspec_root)?;
    let codex_count = count_codex(duckspec_root, canonical_root)?;
    let has_project = duckspec_root.join("project.md").is_file();

    let mut parts = Vec::new();
    if has_project {
        parts.push("project".to_string());
    }
    parts.push(format!(
        "{} {}",
        cap_count,
        if cap_count == 1 { "capability" } else { "capabilities" }
    ));
    parts.push(format!(
        "{} codex {}",
        codex_count,
        if codex_count == 1 { "entry" } else { "entries" }
    ));

    eprintln!("  {}", parts.join(", "));
    eprintln!();

    let changes_dir = duckspec_root.join("changes");
    let change_names = list_subdirs(&changes_dir)?;

    if change_names.is_empty() {
        eprintln!("  {}", "No active changes.".dimmed());
    } else {
        eprintln!("  {}", "Active changes:".bold());
        for name in &change_names {
            let change_dir = changes_dir.join(name);
            let summary = summarize_change(&change_dir, canonical_root)?;
            eprintln!("    {} {}", name.bold(), format!("— {summary}").dimmed());
        }
    }

    Ok(())
}

fn summarize_change(change_dir: &Path, canonical_root: &Path) -> anyhow::Result<String> {
    let files = collect_files(change_dir)?;
    let mut deltas = 0usize;
    let mut new_caps = 0usize;
    let mut steps = 0usize;
    let mut has_proposal = false;
    let mut has_design = false;

    for file_path in &files {
        let canonical = file_path.canonicalize().unwrap_or_else(|_| file_path.clone());
        let Some(relative) = canonical.strip_prefix(canonical_root).ok() else { continue };
        let Some(kind) = layout::classify(relative) else { continue };
        match kind {
            ArtifactKind::SpecDelta | ArtifactKind::DocDelta => deltas += 1,
            ArtifactKind::ChangeCapSpec | ArtifactKind::ChangeCapDoc => new_caps += 1,
            ArtifactKind::Step => steps += 1,
            ArtifactKind::Proposal => has_proposal = true,
            ArtifactKind::Design => has_design = true,
            _ => {}
        }
    }

    let mut parts = Vec::new();
    if has_proposal { parts.push("proposal".to_string()); }
    if has_design { parts.push("design".to_string()); }
    if deltas > 0 { parts.push(format!("{deltas} {}", if deltas == 1 { "delta" } else { "deltas" })); }
    if new_caps > 0 { parts.push(format!("{new_caps} new {}", if new_caps == 1 { "cap" } else { "caps" })); }
    if steps > 0 { parts.push(format!("{steps} {}", if steps == 1 { "step" } else { "steps" })); }

    if parts.is_empty() { Ok("empty".to_string()) } else { Ok(parts.join(", ")) }
}

// ===========================================================================
// Path-specific status
// ===========================================================================

fn run_path_status(
    duckspec_root: &Path,
    canonical_root: &Path,
    input: &str,
) -> anyhow::Result<()> {
    let resolved = resolve_path(input, duckspec_root)
        .or_else(|_| {
            let under_changes = duckspec_root.join("changes").join(input);
            if under_changes.is_dir() { Ok(under_changes) }
            else { anyhow::bail!("not found: {input}") }
        })?;

    let canonical = resolved.canonicalize()?;
    let Some(relative) = canonical.strip_prefix(canonical_root).ok() else {
        anyhow::bail!("{} is not under duckspec/", resolved.display());
    };

    // Detect what kind of target this is.
    if resolved.is_file() {
        let Some(kind) = layout::classify(relative) else {
            anyhow::bail!("unrecognized artifact: {}", relative.display());
        };
        match kind {
            ArtifactKind::CapSpec | ArtifactKind::ChangeCapSpec => {
                return status_spec(&resolved, relative);
            }
            ArtifactKind::SpecDelta => {
                return status_spec_delta(duckspec_root, &resolved, relative);
            }
            ArtifactKind::Step => {
                return status_step(&resolved, relative);
            }
            _ => {
                anyhow::bail!("status not supported for {}", relative.display());
            }
        }
    }

    // Directory — check if it's a change, a steps dir, or a cap dir.
    let components: Vec<&str> = relative
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    // changes/<name>
    if components.len() == 2 && components[0] == "changes" {
        return status_change(duckspec_root, canonical_root, &resolved, components[1]);
    }

    // changes/<name>/steps
    if components.len() == 3 && components[0] == "changes" && components[2] == "steps" {
        return status_steps_dir(&resolved, canonical_root);
    }

    // caps/<path> — show spec status if spec.md exists
    if components.first() == Some(&"caps") && resolved.join("spec.md").is_file() {
        return status_spec(&resolved.join("spec.md"), &relative.join("spec.md"));
    }

    anyhow::bail!("status not supported for {}", relative.display());
}

// ===========================================================================
// Change status
// ===========================================================================

fn status_change(
    duckspec_root: &Path,
    canonical_root: &Path,
    change_dir: &Path,
    change_name: &str,
) -> anyhow::Result<()> {
    let files = collect_files(change_dir)?;

    let mut deltas = 0usize;
    let mut new_caps = 0usize;
    let mut step_files = Vec::new();
    let mut has_proposal = false;
    let mut has_design = false;
    let mut spec_files = Vec::new();

    for file_path in &files {
        let canonical = file_path.canonicalize().unwrap_or_else(|_| file_path.clone());
        let Some(relative) = canonical.strip_prefix(canonical_root).ok() else { continue };
        let Some(kind) = layout::classify(relative) else { continue };
        match kind {
            ArtifactKind::SpecDelta | ArtifactKind::DocDelta => deltas += 1,
            ArtifactKind::ChangeCapSpec => {
                new_caps += 1;
                spec_files.push((file_path.clone(), relative.to_path_buf()));
            }
            ArtifactKind::ChangeCapDoc => new_caps += 1,
            ArtifactKind::Step => step_files.push((file_path.clone(), relative.to_path_buf())),
            ArtifactKind::Proposal => has_proposal = true,
            ArtifactKind::Design => has_design = true,
            _ => {}
        }
    }

    // Header
    eprintln!("  {}", format!("change: {change_name}").bold());
    let mut parts = Vec::new();
    if has_proposal { parts.push("proposal"); }
    if has_design { parts.push("design"); }
    if !parts.is_empty() {
        eprintln!("    {}", parts.join(", ").dimmed());
    }
    if deltas > 0 || new_caps > 0 {
        let mut cap_parts = Vec::new();
        if deltas > 0 { cap_parts.push(format!("{deltas} {}", if deltas == 1 { "delta" } else { "deltas" })); }
        if new_caps > 0 { cap_parts.push(format!("{new_caps} new {}", if new_caps == 1 { "cap" } else { "caps" })); }
        eprintln!("    {}", cap_parts.join(", ").dimmed());
    }

    // Spec coverage — collect scenarios needing backlinks from new cap specs
    let mut needs_backlink = Vec::new();
    let mut covered = 0usize;
    let mut total_scenarios = 0usize;

    for (file_path, relative) in &spec_files {
        let source = std::fs::read_to_string(file_path)?;
        let cap_path = extract_change_cap_path(relative);
        collect_spec_coverage(&source, &cap_path, &mut needs_backlink, &mut covered, &mut total_scenarios);
    }

    // Also check deltas — merge and find new scenarios.
    let caps_dir = duckspec_root.join("caps");
    for file_path in &collect_files(change_dir)? {
        let canonical = file_path.canonicalize().unwrap_or_else(|_| file_path.clone());
        let Some(relative) = canonical.strip_prefix(canonical_root).ok() else { continue };
        if layout::classify(relative) != Some(ArtifactKind::SpecDelta) { continue; }

        let delta_source = std::fs::read_to_string(file_path)?;
        let cap_path = extract_change_cap_path(relative);
        let target = caps_dir.join(&cap_path).join("spec.md");
        if !target.is_file() { continue; }

        let source = std::fs::read_to_string(&target)?;
        let (new_needs, new_covered, _new_total) =
            delta_new_coverage(&source, &delta_source, &cap_path);
        covered += new_covered;
        total_scenarios += new_covered + new_needs.len();
        needs_backlink.extend(new_needs);
    }

    if total_scenarios > 0 {
        eprintln!();
        eprintln!("    {}", "test coverage:".bold());
        if covered > 0 || !needs_backlink.is_empty() {
            let total = covered + needs_backlink.len();
            eprintln!(
                "      {}/{} scenarios with backlinks",
                covered.to_string().green(),
                total
            );
        }
        if !needs_backlink.is_empty() {
            eprintln!("      {}", "missing:".dimmed());
            for tag in &needs_backlink {
                eprintln!("        {}", format!("@spec {tag}").yellow());
            }
        }
    }

    // Steps
    if !step_files.is_empty() {
        eprintln!();
        eprintln!("    {}", "steps:".bold());
        step_files.sort_by(|a, b| a.1.cmp(&b.1));
        for (file_path, relative) in &step_files {
            print_step_summary(file_path, relative, "      ")?;
        }
    }

    Ok(())
}

// ===========================================================================
// Spec status
// ===========================================================================

fn status_spec(file_path: &Path, relative: &Path) -> anyhow::Result<()> {
    let source = std::fs::read_to_string(file_path)?;
    let elements = parse::parse_elements(&source);
    let spec = parse::spec::parse_spec(&elements)
        .map_err(|e| anyhow::anyhow!("parse error: {}", e.first().map(|e| e.to_string()).unwrap_or_default()))?;

    eprintln!("  {} {}", relative.display().to_string().bold(), format!("— {}", spec.title).dimmed());
    eprintln!("    {} requirements, {} scenarios",
        spec.requirements.len(),
        spec.requirements.iter().map(|r| r.scenarios.len()).sum::<usize>()
    );

    // Determine cap_path for display
    let cap_path = extract_cap_path_from_relative(relative);

    let mut needs_backlink = Vec::new();
    let mut covered = 0usize;
    let mut total = 0usize;
    collect_spec_coverage(&source, &cap_path, &mut needs_backlink, &mut covered, &mut total);

    if total > 0 {
        eprintln!(
            "    {}/{} test:code scenarios with backlinks",
            covered.to_string().green(),
            total
        );
        if !needs_backlink.is_empty() {
            eprintln!("    {}", "missing:".dimmed());
            for tag in &needs_backlink {
                eprintln!("      {}", format!("@spec {tag}").yellow());
            }
        }
    }

    Ok(())
}

// ===========================================================================
// Spec delta status
// ===========================================================================

fn status_spec_delta(
    duckspec_root: &Path,
    file_path: &Path,
    relative: &Path,
) -> anyhow::Result<()> {
    let delta_source = std::fs::read_to_string(file_path)?;
    let cap_path = extract_cap_path_from_relative(relative);
    let target = duckspec_root.join("caps").join(&cap_path).join("spec.md");

    eprintln!("  {} {}", relative.display().to_string().bold(), format!("— delta for {cap_path}").dimmed());

    if !target.is_file() {
        eprintln!("    {} target spec not found: caps/{}/spec.md", "!".yellow(), cap_path);
        return Ok(());
    }

    let source = std::fs::read_to_string(&target)?;
    let (new_needs, new_covered, new_total) = delta_new_coverage(&source, &delta_source, &cap_path);

    let added = new_covered + new_needs.len();
    if added > 0 {
        eprintln!("    {} new scenarios", added);
    }

    if new_total > 0 {
        eprintln!(
            "    {}/{} test:code scenarios with backlinks",
            new_covered.to_string().green(),
            new_total
        );
        if !new_needs.is_empty() {
            eprintln!("    {}", "missing:".dimmed());
            for tag in &new_needs {
                eprintln!("      {}", format!("@spec {tag}").yellow());
            }
        }
    }

    Ok(())
}

/// Compute coverage for scenarios a delta introduces (not already in source).
/// Returns (needs_backlink, covered, total_test_code).
fn delta_new_coverage(
    source: &str,
    delta_source: &str,
    cap_path: &str,
) -> (Vec<String>, usize, usize) {
    let mut orig_scenarios = std::collections::HashSet::new();
    collect_scenario_names(source, &mut orig_scenarios);

    let merged = match merge::apply_delta(source, delta_source) {
        Ok(Some(m)) => m,
        _ => return (Vec::new(), 0, 0),
    };

    let mut all_needs = Vec::new();
    let mut all_covered = 0usize;
    let mut all_total = 0usize;
    collect_spec_coverage(&merged, cap_path, &mut all_needs, &mut all_covered, &mut all_total);

    // Also collect coverage for scenarios that ARE in the original to subtract them.
    let mut orig_needs = Vec::new();
    let mut orig_covered = 0usize;
    let mut orig_total = 0usize;
    collect_spec_coverage(source, cap_path, &mut orig_needs, &mut orig_covered, &mut orig_total);

    // Filter to only new scenarios by diffing.
    let orig_tags: std::collections::HashSet<&str> = orig_needs.iter().map(|s| s.as_str()).collect();

    let new_needs: Vec<String> = all_needs
        .into_iter()
        .filter(|tag| !orig_tags.contains(tag.as_str()))
        .filter(|tag| {
            // Also exclude if the scenario name existed in original
            let scn = tag.rsplit(": ").next().unwrap_or("");
            !orig_scenarios.contains(scn)
        })
        .collect();

    let new_covered = all_covered.saturating_sub(orig_covered);
    let new_total = new_covered + new_needs.len();

    (new_needs, new_covered, new_total)
}

// ===========================================================================
// Step status
// ===========================================================================

fn status_step(file_path: &Path, relative: &Path) -> anyhow::Result<()> {
    print_step_summary(file_path, relative, "  ")?;
    Ok(())
}

fn status_steps_dir(dir: &Path, canonical_root: &Path) -> anyhow::Result<()> {
    let files = collect_files(dir)?;
    let mut step_files: Vec<_> = files
        .iter()
        .filter_map(|f| {
            let canonical = f.canonicalize().ok()?;
            let relative = canonical.strip_prefix(canonical_root).ok()?.to_path_buf();
            if layout::classify(&relative) == Some(ArtifactKind::Step) {
                Some((f.clone(), relative))
            } else {
                None
            }
        })
        .collect();
    step_files.sort_by(|a, b| a.1.cmp(&b.1));

    if step_files.is_empty() {
        eprintln!("  {}", "No steps.".dimmed());
        return Ok(());
    }

    for (file_path, relative) in &step_files {
        print_step_summary(file_path, relative, "  ")?;
    }

    Ok(())
}

fn print_step_summary(file_path: &Path, relative: &Path, indent: &str) -> anyhow::Result<()> {
    let source = std::fs::read_to_string(file_path)?;
    let elements = parse::parse_elements(&source);
    let Ok(step) = parse::step::parse_step(&elements) else {
        let filename = relative.file_name().and_then(|f| f.to_str()).unwrap_or("?");
        eprintln!("{indent}{} {}", filename.bold(), "(parse error)".red());
        return Ok(());
    };

    let total_tasks = step.tasks.len()
        + step.tasks.iter().map(|t| t.subtasks.len()).sum::<usize>();
    let done_tasks = step.tasks.iter().filter(|t| t.checked).count()
        + step.tasks.iter().flat_map(|t| &t.subtasks).filter(|s| s.checked).count();

    let filename = relative.file_name().and_then(|f| f.to_str()).unwrap_or("?");

    let progress = if total_tasks == 0 {
        "no tasks".dimmed().to_string()
    } else if done_tasks == total_tasks {
        format!("{done_tasks}/{total_tasks} tasks").green().to_string()
    } else {
        format!("{done_tasks}/{total_tasks} tasks").to_string()
    };

    eprintln!(
        "{indent}{} {} {}",
        filename.bold(),
        format!("— {}", step.title).dimmed(),
        format!("({progress})").dimmed()
    );

    Ok(())
}

// ===========================================================================
// Helpers
// ===========================================================================

fn collect_spec_coverage(
    source: &str,
    cap_path: &str,
    needs_backlink: &mut Vec<String>,
    covered: &mut usize,
    total: &mut usize,
) {
    let elements = parse::parse_elements(source);
    let Ok(spec) = parse::spec::parse_spec(&elements) else { return };

    for req in &spec.requirements {
        for scn in &req.scenarios {
            let is_test_code = scn
                .test_marker
                .as_ref()
                .is_some_and(|m| matches!(m.kind, TestMarkerKind::Code { .. }))
                || req
                    .test_marker
                    .as_ref()
                    .is_some_and(|m| matches!(m.kind, TestMarkerKind::Code { .. }));

            if !is_test_code { continue; }

            *total += 1;

            let has_links = scn.test_marker.as_ref().is_some_and(|m| {
                matches!(&m.kind, TestMarkerKind::Code { backlinks } if !backlinks.is_empty())
            });

            if has_links {
                *covered += 1;
            } else {
                needs_backlink.push(format!("{cap_path} {}: {}", req.name, scn.name));
            }
        }
    }
}

fn collect_scenario_names(source: &str, names: &mut std::collections::HashSet<String>) {
    let elements = parse::parse_elements(source);
    let Ok(spec) = parse::spec::parse_spec(&elements) else { return };
    for req in &spec.requirements {
        for scn in &req.scenarios {
            names.insert(scn.name.clone());
        }
    }
}

/// Extract cap path from a relative path under caps/.
/// e.g. "caps/auth/oauth/spec.md" → "auth/oauth"
fn extract_cap_path_from_relative(relative: &Path) -> String {
    let components: Vec<&str> = relative
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    // Handle both "caps/auth/spec.md" and "changes/x/caps/auth/spec.md"
    if let Some(caps_idx) = components.iter().position(|c| *c == "caps")
        && caps_idx + 2 <= components.len() {
            return components[caps_idx + 1..components.len() - 1].join("/");
        }
    String::new()
}

/// Extract cap path from a change file path.
fn extract_change_cap_path(relative: &Path) -> String {
    extract_cap_path_from_relative(relative)
}

fn count_caps(duckspec_root: &Path) -> anyhow::Result<usize> {
    let caps_dir = duckspec_root.join("caps");
    if !caps_dir.is_dir() { return Ok(0); }
    let mut count = 0;
    count_caps_recursive(&caps_dir, &mut count)?;
    Ok(count)
}

fn count_caps_recursive(dir: &Path, count: &mut usize) -> anyhow::Result<()> {
    if dir.join("spec.md").is_file() || dir.join("doc.md").is_file() {
        *count += 1;
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().is_dir() {
            count_caps_recursive(&entry.path(), count)?;
        }
    }
    Ok(())
}

fn count_codex(duckspec_root: &Path, canonical_root: &Path) -> anyhow::Result<usize> {
    let codex_dir = duckspec_root.join("codex");
    if !codex_dir.is_dir() { return Ok(0); }
    let files = collect_files(&codex_dir)?;
    Ok(files.iter().filter(|f| {
        f.canonicalize().ok()
            .and_then(|c| c.strip_prefix(canonical_root).ok().map(|r| r.to_path_buf()))
            .and_then(|r| layout::classify(&r))
            .is_some_and(|k| k == ArtifactKind::Codex)
    }).count())
}
