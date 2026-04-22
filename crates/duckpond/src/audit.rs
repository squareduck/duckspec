//! Full-project audit: artifact validation, backlink resolution, and
//! test-coverage checks.
//!
//! This module extracts the orchestration of the `ds audit` command into a
//! reusable library function. Consumers (the CLI, the GUI) call
//! [`run_audit`] and render the returned [`AuditReport`] however they like.
//!
//! The audit performs:
//! 1. Per-artifact schema validation over the entire duckspec tree (or a
//!    single change when scoped).
//! 2. Cross-file change validation via [`crate::check::check_change`] for
//!    every active change.
//! 3. A scan of source files for `@spec` backlinks and verification that
//!    every backlink resolves to a scenario.
//! 4. Confirmation that every `test:code` scenario has at least one source
//!    backlink.
//! 5. For each active change, confirmation that every `test:code` scenario
//!    is referenced by at least one step task.
//! 6. Confirmation that every step `@spec` task reference resolves to a
//!    known scenario (considering change-introduced scenarios).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::artifact::spec::{Requirement, Scenario, Spec, TestMarkerKind};
use crate::artifact::step::TaskContent;
use crate::backlink::{self, SourceBacklink};
use crate::check::{self, CheckContext, DuckspecState, LoadedFile};
use crate::config::Config;
use crate::error::{ChangeError, ParseError};
use crate::layout::{self, ArtifactKind};
use crate::merge;
use crate::parse;

// ---------------------------------------------------------------------------
// Report types
// ---------------------------------------------------------------------------

/// Top-level audit result.
#[derive(Debug, Default)]
pub struct AuditReport {
    /// Per-file schema errors found anywhere in the duckspec tree.
    pub artifact_errors: Vec<ArtifactErrorGroup>,
    /// Per-change cross-file errors plus any per-file errors that fall
    /// under `changes/<name>/...`.
    pub change_errors: Vec<ChangeErrorGroup>,
    /// Source-file `@spec` backlinks that do not resolve to any known scenario.
    pub unresolved_backlinks: Vec<UnresolvedBacklink>,
    /// `test:code` scenarios with no source backlink.
    pub missing_backlink_scenarios: Vec<ScenarioKey>,
    /// Per-change: `test:code` scenarios not covered by a step task.
    pub missing_step_coverage: Vec<MissingStepCoverage>,
    /// Step `@spec` task refs that do not resolve to any known scenario.
    pub unresolved_step_refs: Vec<UnresolvedStepRef>,
}

impl AuditReport {
    pub fn total_errors(&self) -> usize {
        let artifact_err_count: usize = self
            .artifact_errors
            .iter()
            .map(|g| g.errors.len())
            .sum();
        let change_err_count: usize = self
            .change_errors
            .iter()
            .map(|g| {
                g.file_errors.iter().map(|(_, _, e)| e.len()).sum::<usize>()
                    + g.change_errors.len()
            })
            .sum();
        let missing_step_count: usize = self
            .missing_step_coverage
            .iter()
            .map(|m| m.missing.len())
            .sum();
        artifact_err_count
            + change_err_count
            + self.unresolved_backlinks.len()
            + self.missing_backlink_scenarios.len()
            + missing_step_count
            + self.unresolved_step_refs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.total_errors() == 0
    }
}

/// All per-file parse errors for one artifact.
#[derive(Debug)]
pub struct ArtifactErrorGroup {
    /// Path relative to the duckspec root.
    pub relative_path: PathBuf,
    /// Source content, retained for `miette` reporting.
    pub source: String,
    pub errors: Vec<ParseError>,
}

/// Per-change error bundle: per-file parse errors within the change plus
/// cross-file errors from [`crate::check::check_change`].
#[derive(Debug)]
pub struct ChangeErrorGroup {
    pub change_name: String,
    /// Per-file errors: (relative path, source, errors).
    pub file_errors: Vec<(PathBuf, String, Vec<ParseError>)>,
    pub change_errors: Vec<ChangeError>,
}

/// Identifies a scenario uniquely across a project.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScenarioKey {
    pub cap_path: String,
    pub requirement: String,
    pub scenario: String,
}

impl ScenarioKey {
    pub fn display(&self) -> String {
        format!("{} {}: {}", self.cap_path, self.requirement, self.scenario)
    }
}

/// A `@spec` backlink in a source file that does not resolve.
#[derive(Debug)]
pub struct UnresolvedBacklink {
    pub source_file: PathBuf,
    pub line: usize,
    pub key: ScenarioKey,
}

/// Step `@spec` task references in a specific change that did not cover all
/// required `test:code` scenarios.
#[derive(Debug)]
pub struct MissingStepCoverage {
    pub change_name: String,
    pub missing: Vec<ScenarioKey>,
}

/// A step `@spec` reference that does not resolve to any known scenario.
#[derive(Debug)]
pub struct UnresolvedStepRef {
    pub change_name: String,
    pub key: ScenarioKey,
}

/// Selects which portion of the project to audit.
#[derive(Debug, Clone)]
pub enum AuditScope {
    /// Audit the full tree: all artifacts, all changes, all backlinks.
    Full,
    /// Audit only the given change plus the global scenario index needed to
    /// resolve its step refs. Artifact validation is limited to files inside
    /// the change.
    Change(String),
}

/// Errors that prevent the audit from completing. Per-artifact and per-change
/// validation errors are returned in [`AuditReport`] instead.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{0} is not under the duckspec root")]
    OutsideDuckspecRoot(PathBuf),
}

impl AuditError {
    fn io(path: &Path, source: std::io::Error) -> Self {
        AuditError::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run a project audit.
///
/// `duckspec_root` must already be canonicalized if callers rely on
/// stripping it from arbitrary paths. This function calls
/// [`Path::canonicalize`] internally to get a stable reference for
/// path-relative operations.
pub fn run_audit(
    duckspec_root: &Path,
    project_root: &Path,
    config: &Config,
    scope: AuditScope,
) -> Result<AuditReport, AuditError> {
    let canonical_root = duckspec_root
        .canonicalize()
        .map_err(|e| AuditError::io(duckspec_root, e))?;

    let mut report = AuditReport::default();

    match &scope {
        AuditScope::Full => {
            audit_full(duckspec_root, &canonical_root, project_root, config, &mut report)?;
        }
        AuditScope::Change(change_name) => {
            audit_change(
                duckspec_root,
                &canonical_root,
                project_root,
                config,
                change_name,
                &mut report,
            )?;
        }
    }

    Ok(report)
}

// ---------------------------------------------------------------------------
// Full audit
// ---------------------------------------------------------------------------

fn audit_full(
    duckspec_root: &Path,
    canonical_root: &Path,
    project_root: &Path,
    config: &Config,
    report: &mut AuditReport,
) -> Result<(), AuditError> {
    // 1. Validate every artifact individually and split results between
    //    per-change groups and project-level groups.
    let mut change_file_errors: HashMap<String, Vec<(PathBuf, String, Vec<ParseError>)>> =
        HashMap::new();
    check_all_artifacts(
        duckspec_root,
        canonical_root,
        &mut report.artifact_errors,
        &mut change_file_errors,
    )?;

    // 2. Cross-file change validation for every active change.
    let state = build_duckspec_state(duckspec_root)?;
    let changes_dir = duckspec_root.join("changes");
    let mut change_names: Vec<String> = Vec::new();
    if changes_dir.is_dir() {
        for entry in read_dir(&changes_dir)? {
            let entry = entry.map_err(|e| AuditError::io(&changes_dir, e))?;
            if entry.path().is_dir()
                && let Some(name) = entry.file_name().to_str()
            {
                change_names.push(name.to_string());
            }
        }
    }
    change_names.sort();

    for change_name in &change_names {
        let loaded = load_change_files(duckspec_root, canonical_root, change_name)?;
        let result = check::check_change(change_name, &loaded, &state);

        // The per-file errors produced by check_change duplicate the
        // artifact-level pass for files inside the change. Use the ones
        // collected during the artifact pass (they have source attached),
        // not the ones returned here.
        let file_errors = change_file_errors.remove(change_name).unwrap_or_default();
        if !file_errors.is_empty() || !result.change_errors.is_empty() {
            report.change_errors.push(ChangeErrorGroup {
                change_name: change_name.clone(),
                file_errors,
                change_errors: result.change_errors,
            });
        }
    }

    // Any files that had errors but whose change was not an active change
    // directory should still surface — keep them under the change name.
    for (change_name, file_errors) in change_file_errors {
        report.change_errors.push(ChangeErrorGroup {
            change_name,
            file_errors,
            change_errors: Vec::new(),
        });
    }
    report
        .change_errors
        .sort_by(|a, b| a.change_name.cmp(&b.change_name));

    // 3. Build the scenario index across all cap specs.
    let scenario_index = build_scenario_index(duckspec_root, canonical_root)?;

    // 4. Scan source files for backlinks.
    let backlinks = scan_source_files(project_root, duckspec_root, config)?;

    // 5. Every backlink must resolve.
    for bl in &backlinks {
        let key = ScenarioKey {
            cap_path: bl.cap_path.clone(),
            requirement: bl.requirement.clone(),
            scenario: bl.scenario.clone(),
        };
        if !scenario_index.contains_key(&key) {
            report.unresolved_backlinks.push(UnresolvedBacklink {
                source_file: bl.file.clone(),
                line: bl.line,
                key,
            });
        }
    }

    // 6. Every test:code scenario must have at least one backlink.
    let backlink_keys = backlink_key_set(&backlinks);
    for (key, is_test_code) in &scenario_index {
        if *is_test_code && !backlink_keys.contains(key) {
            report.missing_backlink_scenarios.push(key.clone());
        }
    }
    report
        .missing_backlink_scenarios
        .sort_by(|a, b| a.display().cmp(&b.display()));

    // 7. For each active change: test:code scenarios covered by step tasks
    //    and step refs resolve.
    for change_name in &change_names {
        let change_dir = changes_dir.join(change_name);
        let change_scenarios =
            build_change_scenarios(duckspec_root, canonical_root, &change_dir)?;
        let test_code: Vec<ScenarioKey> = change_scenarios
            .iter()
            .filter(|s| s.test_code)
            .map(|s| s.key.clone())
            .collect();

        let step_refs = collect_step_refs(duckspec_root, canonical_root, change_name)?;

        if !test_code.is_empty() {
            let ref_set: HashSet<&ScenarioKey> = step_refs.iter().collect();
            let missing: Vec<ScenarioKey> = test_code
                .iter()
                .filter(|k| !ref_set.contains(*k))
                .cloned()
                .collect();
            if !missing.is_empty() {
                report.missing_step_coverage.push(MissingStepCoverage {
                    change_name: change_name.clone(),
                    missing,
                });
            }
        }

        // Step refs must resolve against the merged scenario set (base +
        // scenarios introduced by this change).
        let mut known: HashSet<ScenarioKey> = scenario_index.keys().cloned().collect();
        for s in &change_scenarios {
            known.insert(s.key.clone());
        }
        for r in &step_refs {
            if !known.contains(r) {
                report.unresolved_step_refs.push(UnresolvedStepRef {
                    change_name: change_name.clone(),
                    key: r.clone(),
                });
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Change-scoped audit
// ---------------------------------------------------------------------------

fn audit_change(
    duckspec_root: &Path,
    canonical_root: &Path,
    project_root: &Path,
    config: &Config,
    change_name: &str,
    report: &mut AuditReport,
) -> Result<(), AuditError> {
    let change_dir = duckspec_root.join("changes").join(change_name);

    // 1. Per-file artifact validation for files inside the change only.
    let mut scratch: HashMap<String, Vec<(PathBuf, String, Vec<ParseError>)>> = HashMap::new();
    check_artifacts_in_dir(
        &change_dir,
        canonical_root,
        &mut report.artifact_errors,
        &mut scratch,
    )?;

    // 2. Cross-file change validation.
    let state = build_duckspec_state(duckspec_root)?;
    let loaded = load_change_files(duckspec_root, canonical_root, change_name)?;
    let result = check::check_change(change_name, &loaded, &state);

    let file_errors = scratch.remove(change_name).unwrap_or_default();
    if !file_errors.is_empty() || !result.change_errors.is_empty() {
        report.change_errors.push(ChangeErrorGroup {
            change_name: change_name.to_string(),
            file_errors,
            change_errors: result.change_errors,
        });
    }

    // 3. Build change scenarios and the full scenario index.
    let change_scenarios = build_change_scenarios(duckspec_root, canonical_root, &change_dir)?;
    let scenario_index = build_scenario_index(duckspec_root, canonical_root)?;

    // 4. Scan source files for backlinks.
    let backlinks = scan_source_files(project_root, duckspec_root, config)?;
    let backlink_keys = backlink_key_set(&backlinks);

    // 5. Every test:code scenario introduced by this change must have a
    //    source backlink.
    for s in &change_scenarios {
        if s.test_code && !backlink_keys.contains(&s.key) {
            report.missing_backlink_scenarios.push(s.key.clone());
        }
    }

    // 6. Every test:code scenario must be covered by a step task.
    let test_code: Vec<ScenarioKey> = change_scenarios
        .iter()
        .filter(|s| s.test_code)
        .map(|s| s.key.clone())
        .collect();
    let step_refs = collect_step_refs(duckspec_root, canonical_root, change_name)?;
    if !test_code.is_empty() {
        let ref_set: HashSet<&ScenarioKey> = step_refs.iter().collect();
        let missing: Vec<ScenarioKey> = test_code
            .iter()
            .filter(|k| !ref_set.contains(*k))
            .cloned()
            .collect();
        if !missing.is_empty() {
            report.missing_step_coverage.push(MissingStepCoverage {
                change_name: change_name.to_string(),
                missing,
            });
        }
    }

    // 7. Step refs resolve.
    let mut known: HashSet<ScenarioKey> = scenario_index.keys().cloned().collect();
    for s in &change_scenarios {
        known.insert(s.key.clone());
    }
    for r in &step_refs {
        if !known.contains(r) {
            report.unresolved_step_refs.push(UnresolvedStepRef {
                change_name: change_name.to_string(),
                key: r.clone(),
            });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Artifact validation
// ---------------------------------------------------------------------------

fn check_all_artifacts(
    duckspec_root: &Path,
    canonical_root: &Path,
    project_groups: &mut Vec<ArtifactErrorGroup>,
    change_groups: &mut HashMap<String, Vec<(PathBuf, String, Vec<ParseError>)>>,
) -> Result<(), AuditError> {
    check_artifacts_in_dir(duckspec_root, canonical_root, project_groups, change_groups)
}

fn check_artifacts_in_dir(
    dir: &Path,
    canonical_root: &Path,
    project_groups: &mut Vec<ArtifactErrorGroup>,
    change_groups: &mut HashMap<String, Vec<(PathBuf, String, Vec<ParseError>)>>,
) -> Result<(), AuditError> {
    let files = collect_md_files(dir)?;

    for file_path in &files {
        let canonical = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Ok(relative) = canonical.strip_prefix(canonical_root) else {
            continue;
        };
        let Some(kind) = layout::classify(relative) else {
            continue;
        };

        let source = std::fs::read_to_string(file_path)
            .map_err(|e| AuditError::io(file_path, e))?;
        let ctx = build_context(&kind, relative);
        let result = check::check_artifact(&source, &kind, &ctx);
        if result.errors.is_empty() {
            continue;
        }

        if let Some(change_name) = change_name_from_relative(relative) {
            change_groups
                .entry(change_name)
                .or_default()
                .push((relative.to_path_buf(), source, result.errors));
        } else {
            project_groups.push(ArtifactErrorGroup {
                relative_path: relative.to_path_buf(),
                source,
                errors: result.errors,
            });
        }
    }

    project_groups.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(())
}

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

fn change_name_from_relative(relative: &Path) -> Option<String> {
    let mut comps = relative.components();
    if comps.next()?.as_os_str() != "changes" {
        return None;
    }
    let name = comps.next()?.as_os_str().to_str()?.to_string();
    Some(name)
}

// ---------------------------------------------------------------------------
// Duckspec state
// ---------------------------------------------------------------------------

fn build_duckspec_state(duckspec_root: &Path) -> Result<DuckspecState, AuditError> {
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
) -> Result<(), AuditError> {
    for entry in read_dir(dir)? {
        let entry = entry.map_err(|e| AuditError::io(dir, e))?;
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

// ---------------------------------------------------------------------------
// Loading change files
// ---------------------------------------------------------------------------

fn load_change_files(
    duckspec_root: &Path,
    canonical_root: &Path,
    change_name: &str,
) -> Result<Vec<LoadedFile>, AuditError> {
    let change_dir = duckspec_root.join("changes").join(change_name);
    let file_paths = collect_md_files(&change_dir)?;
    let mut loaded = Vec::new();

    for file_path in &file_paths {
        let canonical = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Ok(relative) = canonical.strip_prefix(canonical_root) else {
            continue;
        };
        let Some(kind) = layout::classify(relative) else {
            continue;
        };
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| AuditError::io(file_path, e))?;
        loaded.push(LoadedFile {
            relative_path: relative.to_path_buf(),
            kind,
            content,
        });
    }

    Ok(loaded)
}

// ---------------------------------------------------------------------------
// Scenario index
// ---------------------------------------------------------------------------

/// Scenario index from top-level cap specs. Key → `is_test_code`.
fn build_scenario_index(
    duckspec_root: &Path,
    canonical_root: &Path,
) -> Result<HashMap<ScenarioKey, bool>, AuditError> {
    let caps_dir = duckspec_root.join("caps");
    if !caps_dir.is_dir() {
        return Ok(HashMap::new());
    }

    let mut index = HashMap::new();
    let files = collect_md_files(&caps_dir)?;

    for file_path in &files {
        let canonical = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Ok(relative) = canonical.strip_prefix(canonical_root) else {
            continue;
        };
        if layout::classify(relative) != Some(ArtifactKind::CapSpec) {
            continue;
        }

        let source = std::fs::read_to_string(file_path)
            .map_err(|e| AuditError::io(file_path, e))?;
        let cap_path = extract_cap_path(relative);
        index_spec_scenarios(&source, &cap_path, &mut index);
    }

    Ok(index)
}

fn extract_cap_path(relative: &Path) -> String {
    let components: Vec<&str> = relative
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    if components.len() >= 3 {
        components[1..components.len() - 1].join("/")
    } else {
        String::new()
    }
}

fn extract_change_cap_path(relative: &Path) -> String {
    let components: Vec<&str> = relative
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    if let Some(caps_idx) = components.iter().position(|c| *c == "caps") {
        components[caps_idx + 1..components.len() - 1].join("/")
    } else {
        String::new()
    }
}

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
            let is_test_code = scenario_is_test_code(req, scn);

            let key = ScenarioKey {
                cap_path: cap_path.to_string(),
                requirement: req.name.clone(),
                scenario: scn.name.clone(),
            };
            index.insert(key, is_test_code);
        }
    }
}

/// Resolve whether a scenario is `test: code`. A scenario's own marker fully
/// overrides the requirement default; only when the scenario has no marker
/// does it inherit.
fn scenario_is_test_code(req: &Requirement, scn: &Scenario) -> bool {
    match &scn.test_marker {
        Some(marker) => matches!(marker.kind, TestMarkerKind::Code { .. }),
        None => req
            .test_marker
            .as_ref()
            .is_some_and(|m| matches!(m.kind, TestMarkerKind::Code { .. })),
    }
}

// ---------------------------------------------------------------------------
// Change scenarios (post-merge)
// ---------------------------------------------------------------------------

struct ChangeScenario {
    key: ScenarioKey,
    test_code: bool,
}

fn build_change_scenarios(
    duckspec_root: &Path,
    canonical_root: &Path,
    change_dir: &Path,
) -> Result<Vec<ChangeScenario>, AuditError> {
    let files = collect_md_files(change_dir)?;
    let caps_dir = duckspec_root.join("caps");
    let mut scenarios = Vec::new();

    for file_path in &files {
        let canonical = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Ok(relative) = canonical.strip_prefix(canonical_root) else {
            continue;
        };
        let Some(kind) = layout::classify(relative) else {
            continue;
        };

        match kind {
            ArtifactKind::ChangeCapSpec => {
                let source = std::fs::read_to_string(file_path)
                    .map_err(|e| AuditError::io(file_path, e))?;
                let cap_path = extract_change_cap_path(relative);
                let elements = parse::parse_elements(&source);
                if let Ok(spec) = parse::spec::parse_spec(&elements) {
                    for req in &spec.requirements {
                        for scn in &req.scenarios {
                            let is_test_code = scenario_is_test_code(req, scn);
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
                let delta_source = std::fs::read_to_string(file_path)
                    .map_err(|e| AuditError::io(file_path, e))?;
                let cap_path = extract_change_cap_path(relative);
                let target_path = caps_dir.join(&cap_path).join("spec.md");

                if target_path.is_file() {
                    let source = std::fs::read_to_string(&target_path)
                        .map_err(|e| AuditError::io(&target_path, e))?;

                    let mut original_index = HashMap::new();
                    index_spec_scenarios(&source, &cap_path, &mut original_index);

                    if let Ok(Some(merged)) = merge::apply_delta(&source, &delta_source) {
                        let elements = parse::parse_elements(&merged);
                        if let Ok(spec) = parse::spec::parse_spec(&elements) {
                            let mut merged_index = HashMap::new();
                            add_spec_to_index(&spec, &cap_path, &mut merged_index);
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

// ---------------------------------------------------------------------------
// Source backlink scanning
// ---------------------------------------------------------------------------

fn scan_source_files(
    project_root: &Path,
    duckspec_root: &Path,
    config: &Config,
) -> Result<Vec<SourceBacklink>, AuditError> {
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

    let duckspec_canonical = duckspec_root
        .canonicalize()
        .map_err(|e| AuditError::io(duckspec_root, e))?;
    let mut all_backlinks = Vec::new();

    for root in &scan_roots {
        let walker = WalkBuilder::new(root).build();

        for result in walker {
            let entry = match result {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if let Ok(canonical) = path.canonicalize()
                && canonical.starts_with(&duckspec_canonical)
            {
                continue;
            }

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

fn backlink_key_set(backlinks: &[SourceBacklink]) -> HashSet<ScenarioKey> {
    backlinks
        .iter()
        .map(|bl| ScenarioKey {
            cap_path: bl.cap_path.clone(),
            requirement: bl.requirement.clone(),
            scenario: bl.scenario.clone(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Step refs
// ---------------------------------------------------------------------------

fn collect_step_refs(
    duckspec_root: &Path,
    canonical_root: &Path,
    change_name: &str,
) -> Result<Vec<ScenarioKey>, AuditError> {
    let steps_dir = duckspec_root
        .join("changes")
        .join(change_name)
        .join("steps");
    if !steps_dir.is_dir() {
        return Ok(Vec::new());
    }

    let files = collect_md_files(&steps_dir)?;
    let mut refs = Vec::new();

    for file_path in &files {
        let canonical = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let Ok(relative) = canonical.strip_prefix(canonical_root) else {
            continue;
        };
        if layout::classify(relative) != Some(ArtifactKind::Step) {
            continue;
        }

        let source = std::fs::read_to_string(file_path)
            .map_err(|e| AuditError::io(file_path, e))?;
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

// ---------------------------------------------------------------------------
// Filesystem helpers
// ---------------------------------------------------------------------------

fn collect_md_files(path: &Path) -> Result<Vec<PathBuf>, AuditError> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    if !path.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    walk_md(path, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_md(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), AuditError> {
    for entry in read_dir(dir)? {
        let entry = entry.map_err(|e| AuditError::io(dir, e))?;
        let path = entry.path();
        if path.is_dir() {
            walk_md(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
    Ok(())
}

fn read_dir(dir: &Path) -> Result<std::fs::ReadDir, AuditError> {
    std::fs::read_dir(dir).map_err(|e| AuditError::io(dir, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find<'a>(spec: &'a Spec, req_name: &str, scn_name: &str) -> (&'a Requirement, &'a Scenario) {
        let req = spec
            .requirements
            .iter()
            .find(|r| r.name == req_name)
            .expect("requirement");
        let scn = req
            .scenarios
            .iter()
            .find(|s| s.name == scn_name)
            .expect("scenario");
        (req, scn)
    }

    #[test]
    fn scenario_level_skip_overrides_inherited_test_code() {
        let source = "\
# X

Summary.

## Requirement: R

> test: code

### Scenario: Inherits code

- **WHEN** something
- **THEN** something else

### Scenario: Overrides with skip

- **WHEN** something
- **THEN** something else

> skip: documented only; redundant with redirect integration test

### Scenario: Overrides with manual

- **WHEN** something
- **THEN** something else

> manual: QA checklist item
";
        let elements = parse::parse_elements(source);
        let spec = parse::spec::parse_spec(&elements).expect("parse");

        let (req, inherits) = find(&spec, "R", "Inherits code");
        assert!(
            scenario_is_test_code(req, inherits),
            "scenario with no marker inherits test:code from requirement"
        );

        let (req, skipped) = find(&spec, "R", "Overrides with skip");
        assert!(
            !scenario_is_test_code(req, skipped),
            "scenario-level skip must override requirement test:code"
        );

        let (req, manual) = find(&spec, "R", "Overrides with manual");
        assert!(
            !scenario_is_test_code(req, manual),
            "scenario-level manual must override requirement test:code"
        );
    }
}
