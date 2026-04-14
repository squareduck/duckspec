use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use duckpond::artifact::spec::{Spec, TestMarkerKind};
use duckpond::artifact::step::TaskContent;
use duckpond::backlink::{self, SourceBacklink};
use duckpond::check::{self, CheckContext, DuckspecState, LoadedFile};
use duckpond::config::Config;
use duckpond::error::ParseError;
use duckpond::layout::{self, ArtifactKind};
use duckpond::merge;
use duckpond::parse;
use ignore::WalkBuilder;
use miette::NamedSource;
use owo_colors::OwoColorize;

use super::common::{collect_files, find_duckspec_root, resolve_path};

pub fn run(change: Option<String>) -> anyhow::Result<()> {
    let duckspec_root = find_duckspec_root()?;

    match change {
        Some(c) => run_change_audit(&duckspec_root, &c),
        None => run_full_audit(&duckspec_root),
    }
}

// ===========================================================================
// Full project audit
// ===========================================================================

fn run_full_audit(duckspec_root: &Path) -> anyhow::Result<()> {
    let canonical_root = duckspec_root.canonicalize()?;
    let project_root = duckspec_root
        .parent()
        .ok_or_else(|| anyhow::anyhow!("duckspec/ has no parent directory"))?;
    let config = Config::load(duckspec_root)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut error_count = 0usize;

    // 1. Artifact validation (ds check over the whole tree).
    eprintln!("  {} checking artifacts…", "·".dimmed());
    error_count += check_all_artifacts(duckspec_root, &canonical_root)?;

    // 2. Build scenario index from all cap specs.
    let scenario_index = build_scenario_index(duckspec_root, &canonical_root)?;

    // 3. Scan source files for @spec backlinks.
    eprintln!("  {} scanning for backlinks…", "·".dimmed());
    let backlinks = scan_source_files(project_root, duckspec_root, &config)?;

    // 4. Verify every backlink resolves.
    error_count += check_backlinks_resolve(&backlinks, &scenario_index, project_root);

    // 5. Every test:code scenario has at least one backlink.
    let test_code_scenarios = collect_test_code_scenarios(&scenario_index);
    error_count += check_scenario_coverage(&test_code_scenarios, &backlinks);

    // 6. For active changes: test:code scenarios are covered by step tasks.
    error_count += check_change_step_coverage(duckspec_root, &canonical_root, &scenario_index)?;

    if error_count > 0 {
        eprintln!();
        eprintln!("{}", format!("{error_count} audit error(s)").red().bold());
        std::process::exit(1);
    }

    eprintln!("  {} audit ok", "✓".green());
    Ok(())
}

// ===========================================================================
// Change-scoped audit
// ===========================================================================

fn run_change_audit(duckspec_root: &Path, input: &str) -> anyhow::Result<()> {
    let canonical_root = duckspec_root.canonicalize()?;
    let project_root = duckspec_root
        .parent()
        .ok_or_else(|| anyhow::anyhow!("duckspec/ has no parent directory"))?;
    let config = Config::load(duckspec_root)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Resolve the change directory.
    let change_dir = resolve_path(input, duckspec_root)
        .or_else(|_| {
            let under_changes = duckspec_root.join("changes").join(input);
            if under_changes.is_dir() {
                Ok(under_changes)
            } else {
                anyhow::bail!("change not found: {input}")
            }
        })?;

    let changes_dir = duckspec_root.join("changes").canonicalize()?;
    let change_canonical = change_dir.canonicalize()?;
    let change_name = change_canonical
        .strip_prefix(&changes_dir)
        .map_err(|_| anyhow::anyhow!("{} is not under changes/", change_dir.display()))?
        .components()
        .next()
        .and_then(|c| c.as_os_str().to_str())
        .ok_or_else(|| anyhow::anyhow!("could not extract change name"))?
        .to_string();

    let mut error_count = 0usize;

    // 1. Validate change artifacts.
    eprintln!("  {} checking change {change_name}…", "·".dimmed());
    error_count += check_change_artifacts(duckspec_root, &canonical_root, &change_name)?;

    // 2. Build the post-merge scenario set for this change.
    let change_scenarios = build_change_scenarios(
        duckspec_root,
        &canonical_root,
        &change_dir,
    )?;

    // 3. Scan source for backlinks.
    eprintln!("  {} scanning for backlinks…", "·".dimmed());
    let all_backlinks = scan_source_files(project_root, duckspec_root, &config)?;

    // 4. Check backlinks that reference this change's scenarios.
    let change_test_code: Vec<ScenarioKey> = change_scenarios
        .iter()
        .filter(|s| s.test_code)
        .map(|s| s.key.clone())
        .collect();
    error_count += check_scenario_coverage(&change_test_code, &all_backlinks);

    // 5. Check step task coverage for the change's test:code scenarios.
    let step_refs = collect_step_refs(duckspec_root, &canonical_root, &change_name)?;
    error_count += check_step_task_coverage(&change_test_code, &step_refs);

    // 6. Check every step @spec task resolves.
    let full_index = build_scenario_index(duckspec_root, &canonical_root)?;
    // Merge with change scenarios for resolution.
    let mut merged_keys: HashSet<ScenarioKey> = full_index.keys().cloned().collect();
    for s in &change_scenarios {
        merged_keys.insert(s.key.clone());
    }
    error_count += check_step_refs_resolve(&step_refs, &merged_keys);

    if error_count > 0 {
        eprintln!();
        eprintln!("{}", format!("{error_count} audit error(s)").red().bold());
        std::process::exit(1);
    }

    eprintln!("  {} audit ok", "✓".green());
    Ok(())
}

// ===========================================================================
// Scenario index
// ===========================================================================

/// Key identifying a unique scenario.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ScenarioKey {
    cap_path: String,
    requirement: String,
    scenario: String,
}

impl ScenarioKey {
    fn display(&self) -> String {
        format!("{} {}: {}", self.cap_path, self.requirement, self.scenario)
    }
}

/// A scenario found during index building, with metadata.
struct ChangeScenario {
    key: ScenarioKey,
    test_code: bool,
}

/// Build an index of all scenarios in top-level cap specs.
/// Key → is_test_code.
fn build_scenario_index(
    duckspec_root: &Path,
    canonical_root: &Path,
) -> anyhow::Result<HashMap<ScenarioKey, bool>> {
    let caps_dir = duckspec_root.join("caps");
    if !caps_dir.is_dir() {
        return Ok(HashMap::new());
    }

    let mut index = HashMap::new();
    let files = collect_files(&caps_dir)?;

    for file_path in &files {
        let canonical = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Some(relative) = canonical.strip_prefix(canonical_root).ok() else {
            continue;
        };
        if layout::classify(relative) != Some(ArtifactKind::CapSpec) {
            continue;
        }

        let source = std::fs::read_to_string(file_path)?;
        let cap_path = extract_cap_path(relative);
        index_spec_scenarios(&source, &cap_path, &mut index);
    }

    Ok(index)
}

/// Extract the capability path from a relative file path.
/// e.g. "caps/auth/oauth/spec.md" → "auth/oauth"
fn extract_cap_path(relative: &Path) -> String {
    let components: Vec<&str> = relative
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    // Skip "caps" prefix and "spec.md" suffix.
    if components.len() >= 3 {
        components[1..components.len() - 1].join("/")
    } else {
        String::new()
    }
}

/// Parse a spec and add its scenarios to the index.
fn index_spec_scenarios(
    source: &str,
    cap_path: &str,
    index: &mut HashMap<ScenarioKey, bool>,
) {
    let elements = parse::parse_elements(source);
    let Ok(spec) = parse::spec::parse_spec(&elements) else {
        return;
    };
    add_spec_to_index(&spec, cap_path, index);
}

fn add_spec_to_index(
    spec: &Spec,
    cap_path: &str,
    index: &mut HashMap<ScenarioKey, bool>,
) {
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

            let key = ScenarioKey {
                cap_path: cap_path.to_string(),
                requirement: req.name.clone(),
                scenario: scn.name.clone(),
            };
            index.insert(key, is_test_code);
        }
    }
}

// ===========================================================================
// Change scenarios (post-merge view)
// ===========================================================================

fn build_change_scenarios(
    duckspec_root: &Path,
    canonical_root: &Path,
    change_dir: &Path,
) -> anyhow::Result<Vec<ChangeScenario>> {
    let files = collect_files(change_dir)?;
    let caps_dir = duckspec_root.join("caps");
    let mut scenarios = Vec::new();

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

        match kind {
            ArtifactKind::ChangeCapSpec => {
                // New cap spec — all its scenarios are introduced.
                let source = std::fs::read_to_string(file_path)?;
                let cap_path = extract_change_cap_path(relative);
                let elements = parse::parse_elements(&source);
                if let Ok(spec) = parse::spec::parse_spec(&elements) {
                    for req in &spec.requirements {
                        for scn in &req.scenarios {
                            let is_test_code = scn
                                .test_marker
                                .as_ref()
                                .is_some_and(|m| matches!(m.kind, TestMarkerKind::Code { .. }))
                                || req
                                    .test_marker
                                    .as_ref()
                                    .is_some_and(|m| {
                                        matches!(m.kind, TestMarkerKind::Code { .. })
                                    });

                            scenarios.push(ChangeScenario {
                                key: ScenarioKey {
                                    cap_path: cap_path.clone(),
                                    requirement: req.name.clone(),
                                    scenario: scn.name.clone(),
                                },
                                test_code: is_test_code,
                            });
                        }
                    }
                }
            }
            ArtifactKind::SpecDelta => {
                // Merge the delta with the existing spec, then diff to find
                // only the scenarios this change introduces.
                let delta_source = std::fs::read_to_string(file_path)?;
                let cap_path = extract_change_cap_path(relative);
                let target_path = caps_dir.join(&cap_path).join("spec.md");

                if target_path.is_file() {
                    let source = std::fs::read_to_string(&target_path)?;

                    // Index the original spec's scenarios.
                    let mut original_index = HashMap::new();
                    index_spec_scenarios(&source, &cap_path, &mut original_index);

                    if let Ok(Some(merged)) = merge::apply_delta(&source, &delta_source) {
                        let elements = parse::parse_elements(&merged);
                        if let Ok(spec) = parse::spec::parse_spec(&elements) {
                            let mut merged_index = HashMap::new();
                            add_spec_to_index(&spec, &cap_path, &mut merged_index);
                            // Only include scenarios that are new (not in original).
                            for (key, is_test_code) in merged_index {
                                if !original_index.contains_key(&key) {
                                    scenarios.push(ChangeScenario {
                                        key,
                                        test_code: is_test_code,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(scenarios)
}

/// Extract cap path from a change file path.
/// e.g. "changes/add-two-factor/caps/auth/two-factor/spec.md" → "auth/two-factor"
fn extract_change_cap_path(relative: &Path) -> String {
    let components: Vec<&str> = relative
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    // changes/<name>/caps/<cap-path...>/spec.md or spec.delta.md
    if let Some(caps_idx) = components.iter().position(|c| *c == "caps") {
        components[caps_idx + 1..components.len() - 1].join("/")
    } else {
        String::new()
    }
}

// ===========================================================================
// Source scanning
// ===========================================================================

fn scan_source_files(
    project_root: &Path,
    duckspec_root: &Path,
    config: &Config,
) -> anyhow::Result<Vec<SourceBacklink>> {
    let scan_roots = if config.test_paths.is_empty() {
        vec![project_root.to_path_buf()]
    } else {
        config
            .test_paths
            .iter()
            .map(|p| project_root.join(p))
            .filter(|p| p.exists())
            .collect()
    };

    let duckspec_canonical = duckspec_root.canonicalize()?;
    let mut all_backlinks = Vec::new();

    for root in &scan_roots {
        let walker = WalkBuilder::new(root).build();

        for result in walker {
            let entry = match result {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();

            // Skip directories and non-files.
            if !path.is_file() {
                continue;
            }

            // Skip files inside duckspec/.
            if let Ok(canonical) = path.canonicalize()
                && canonical.starts_with(&duckspec_canonical) {
                    continue;
                }

            // Skip binary files — try reading as UTF-8.
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let found = backlink::scan_file(path, &content);
            all_backlinks.extend(found);
        }
    }

    Ok(all_backlinks)
}

// ===========================================================================
// Checks
// ===========================================================================

/// Run artifact validation over the entire duckspec tree.
fn check_all_artifacts(
    duckspec_root: &Path,
    canonical_root: &Path,
) -> anyhow::Result<usize> {
    let files = collect_files(duckspec_root)?;
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

        if !result.errors.is_empty() {
            let named = NamedSource::new(relative.display().to_string(), source.clone());
            report_parse_errors(&result.errors, named);
            error_count += result.errors.len();
        }
    }

    Ok(error_count)
}

/// Run change-level validation.
fn check_change_artifacts(
    duckspec_root: &Path,
    canonical_root: &Path,
    change_name: &str,
) -> anyhow::Result<usize> {
    let change_dir = duckspec_root.join("changes").join(change_name);
    let file_paths = collect_files(&change_dir)?;
    let mut loaded_files = Vec::new();

    for file_path in &file_paths {
        let canonical = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Some(relative) = canonical.strip_prefix(canonical_root).ok() else {
            continue;
        };
        let Some(kind) = layout::classify(relative) else {
            continue;
        };
        let content = std::fs::read_to_string(file_path)?;
        loaded_files.push(LoadedFile {
            relative_path: relative.to_path_buf(),
            kind,
            content,
        });
    }

    let state = build_duckspec_state(duckspec_root)?;
    let result = check::check_change(change_name, &loaded_files, &state);

    let mut error_count = 0usize;

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
                eprintln!("  {} {}: {err}", "×".red(), path.display());
            }
        }
        error_count += errors.len();
    }
    for err in &result.change_errors {
        eprintln!("  {} {err}", "×".red());
        error_count += 1;
    }

    Ok(error_count)
}

/// Check that every backlink resolves to an existing scenario.
fn check_backlinks_resolve(
    backlinks: &[SourceBacklink],
    scenario_index: &HashMap<ScenarioKey, bool>,
    project_root: &Path,
) -> usize {
    let mut errors = 0;
    for bl in backlinks {
        let key = ScenarioKey {
            cap_path: bl.cap_path.clone(),
            requirement: bl.requirement.clone(),
            scenario: bl.scenario.clone(),
        };
        if !scenario_index.contains_key(&key) {
            let display_path = bl.file.strip_prefix(project_root).unwrap_or(&bl.file);
            eprintln!(
                "  {} {}:{} — backlink does not resolve: {}",
                "×".red(),
                display_path.display(),
                bl.line,
                key.display()
            );
            errors += 1;
        }
    }
    errors
}

/// Collect all test:code scenario keys.
fn collect_test_code_scenarios(
    index: &HashMap<ScenarioKey, bool>,
) -> Vec<ScenarioKey> {
    index
        .iter()
        .filter(|(_, is_test_code)| **is_test_code)
        .map(|(k, _)| k.clone())
        .collect()
}

/// Check that every test:code scenario has at least one backlink.
fn check_scenario_coverage(
    test_code_scenarios: &[ScenarioKey],
    backlinks: &[SourceBacklink],
) -> usize {
    let backlink_keys: HashSet<ScenarioKey> = backlinks
        .iter()
        .map(|bl| ScenarioKey {
            cap_path: bl.cap_path.clone(),
            requirement: bl.requirement.clone(),
            scenario: bl.scenario.clone(),
        })
        .collect();

    let mut errors = 0;
    for key in test_code_scenarios {
        if !backlink_keys.contains(key) {
            eprintln!(
                "  {} scenario marked test:code has no backlink: {}",
                "×".red(),
                key.display()
            );
            errors += 1;
        }
    }
    errors
}

/// Check that each active change's test:code scenarios have step tasks.
fn check_change_step_coverage(
    duckspec_root: &Path,
    canonical_root: &Path,
    _scenario_index: &HashMap<ScenarioKey, bool>,
) -> anyhow::Result<usize> {
    let changes_dir = duckspec_root.join("changes");
    if !changes_dir.is_dir() {
        return Ok(0);
    }

    let mut total_errors = 0;

    for entry in std::fs::read_dir(&changes_dir)? {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }
        let change_name = match entry.file_name().to_str() {
            Some(n) => n.to_string(),
            None => continue,
        };

        let change_dir = changes_dir.join(&change_name);
        let change_scenarios = build_change_scenarios(
            duckspec_root,
            canonical_root,
            &change_dir,
        )?;

        let test_code: Vec<ScenarioKey> = change_scenarios
            .iter()
            .filter(|s| s.test_code)
            .map(|s| s.key.clone())
            .collect();

        if test_code.is_empty() {
            continue;
        }

        let step_refs = collect_step_refs(duckspec_root, canonical_root, &change_name)?;
        total_errors += check_step_task_coverage(&test_code, &step_refs);
    }

    Ok(total_errors)
}

/// Collect @spec task references from a change's steps.
fn collect_step_refs(
    duckspec_root: &Path,
    canonical_root: &Path,
    change_name: &str,
) -> anyhow::Result<Vec<ScenarioKey>> {
    let steps_dir = duckspec_root
        .join("changes")
        .join(change_name)
        .join("steps");
    if !steps_dir.is_dir() {
        return Ok(Vec::new());
    }

    let files = collect_files(&steps_dir)?;
    let mut refs = Vec::new();

    for file_path in &files {
        let canonical = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Some(relative) = canonical.strip_prefix(canonical_root).ok() else {
            continue;
        };
        if layout::classify(relative) != Some(ArtifactKind::Step) {
            continue;
        }

        let source = std::fs::read_to_string(file_path)?;
        let elements = parse::parse_elements(&source);
        let Ok(step) = parse::step::parse_step(&elements) else {
            continue;
        };

        for task in &step.tasks {
            if let TaskContent::SpecRef {
                capability,
                requirement,
                scenario,
            } = &task.content
            {
                refs.push(ScenarioKey {
                    cap_path: capability.clone(),
                    requirement: requirement.clone(),
                    scenario: scenario.clone(),
                });
            }
            for sub in &task.subtasks {
                if let TaskContent::SpecRef {
                    capability,
                    requirement,
                    scenario,
                } = &sub.content
                {
                    refs.push(ScenarioKey {
                        cap_path: capability.clone(),
                        requirement: requirement.clone(),
                        scenario: scenario.clone(),
                    });
                }
            }
        }
    }

    Ok(refs)
}

/// Check that every test:code scenario has a step task covering it.
fn check_step_task_coverage(
    test_code_scenarios: &[ScenarioKey],
    step_refs: &[ScenarioKey],
) -> usize {
    let ref_set: HashSet<&ScenarioKey> = step_refs.iter().collect();
    let mut errors = 0;
    for key in test_code_scenarios {
        if !ref_set.contains(key) {
            eprintln!(
                "  {} test:code scenario not covered by step task: {}",
                "×".red(),
                key.display()
            );
            errors += 1;
        }
    }
    errors
}

/// Check that every step @spec ref resolves to an existing scenario.
fn check_step_refs_resolve(
    step_refs: &[ScenarioKey],
    known_scenarios: &HashSet<ScenarioKey>,
) -> usize {
    let mut errors = 0;
    for key in step_refs {
        if !known_scenarios.contains(key) {
            eprintln!(
                "  {} step @spec task does not resolve: {}",
                "×".red(),
                key.display()
            );
            errors += 1;
        }
    }
    errors
}

// ===========================================================================
// Helpers
// ===========================================================================

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
    spec_paths: &mut HashSet<PathBuf>,
    doc_paths: &mut HashSet<PathBuf>,
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

fn report_parse_errors(errors: &[ParseError], source: NamedSource<String>) {
    for err in errors {
        let report = miette::Report::new(err.clone()).with_source_code(source.clone());
        eprintln!("{report:?}");
    }
}
