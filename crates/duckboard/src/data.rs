//! Project data model and filesystem loader.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ProjectData {
    pub project_root: Option<PathBuf>,
    pub duckspec_root: Option<PathBuf>,
    pub active_changes: Vec<ChangeData>,
    pub archived_changes: Vec<ChangeData>,
    pub cap_tree: Vec<TreeNode>,
    pub codex_entries: Vec<TreeNode>,
    /// Per-change validation results, populated on audit.
    pub validations: HashMap<String, ChangeValidation>,
    /// Project-level audit findings (artifact errors outside changes,
    /// backlink/coverage issues), populated on audit.
    pub project_audit: ProjectAudit,
}

/// Validation results for a single change.
#[derive(Debug, Clone, Default)]
pub struct ChangeValidation {
    /// Per-file parse errors: (relative path, list of error messages).
    pub file_errors: Vec<(String, Vec<String>)>,
    /// Cross-file change-level errors.
    pub change_errors: Vec<String>,
}

impl ChangeValidation {
    pub fn total_count(&self) -> usize {
        self.file_errors
            .iter()
            .map(|(_, errs)| errs.len())
            .sum::<usize>()
            + self.change_errors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.file_errors.is_empty() && self.change_errors.is_empty()
    }
}

/// Audit findings that are not scoped to a single change.
#[derive(Debug, Clone, Default)]
pub struct ProjectAudit {
    /// Per-file parse errors outside `changes/` (under `caps/`, `codex/`, etc.).
    pub artifact_errors: Vec<(String, Vec<String>)>,
    /// `@spec` backlinks in source files that do not resolve.
    pub unresolved_backlinks: Vec<BacklinkIssue>,
    /// `test:code` scenarios that have no source backlink.
    pub missing_backlink_scenarios: Vec<String>,
    /// Per-change `test:code` scenarios not covered by a step task.
    pub missing_step_coverage: Vec<(String, Vec<String>)>,
    /// Step `@spec` refs that do not resolve.
    pub unresolved_step_refs: Vec<(String, String)>,
}

impl ProjectAudit {
    pub fn total_count(&self) -> usize {
        let artifact: usize = self.artifact_errors.iter().map(|(_, e)| e.len()).sum();
        let coverage: usize = self
            .missing_step_coverage
            .iter()
            .map(|(_, s)| s.len())
            .sum();
        artifact
            + self.unresolved_backlinks.len()
            + self.missing_backlink_scenarios.len()
            + coverage
            + self.unresolved_step_refs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.total_count() == 0
    }
}

/// An unresolved `@spec` backlink found in source code.
#[derive(Debug, Clone)]
pub struct BacklinkIssue {
    /// Path relative to the project root when possible, absolute otherwise.
    pub source_path: String,
    pub line: usize,
    /// `cap_path Requirement: Scenario` form.
    pub scenario_display: String,
}

#[derive(Debug, Clone)]
pub struct ChangeData {
    pub name: String,
    pub prefix: String,
    pub has_proposal: bool,
    pub has_design: bool,
    pub cap_tree: Vec<TreeNode>,
    pub steps: Vec<StepInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepCompletion {
    /// No tasks defined in the step file.
    NoTasks,
    /// Some tasks remain (done, total).
    Partial(usize, usize),
    /// All tasks complete.
    Done,
}

#[derive(Debug, Clone)]
pub struct StepInfo {
    pub id: String,
    pub label: String,
    pub completion: StepCompletion,
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    /// Collect the IDs of all non-leaf nodes in the tree (recursively).
    pub fn collect_parent_ids(nodes: &[TreeNode], out: &mut std::collections::HashSet<String>) {
        for node in nodes {
            if !node.children.is_empty() {
                out.insert(node.id.clone());
                Self::collect_parent_ids(&node.children, out);
            }
        }
    }
}

// ── Loading ──────────────────────────────────────────────────────────────────

impl ProjectData {
    /// Open the project rooted at `project_root`. If a `duckspec/` directory
    /// exists inside it, load artifacts; otherwise keep the roots set so
    /// duckboard still "has" a project but displays empty panels.
    pub fn open(project_root: &Path) -> Self {
        let duckspec = project_root.join("duckspec");
        let duckspec_root = if duckspec.is_dir() {
            Some(duckspec)
        } else {
            None
        };
        Self::load_from(project_root, duckspec_root.as_deref())
    }

    fn load_from(project_root: &Path, duckspec_root: Option<&Path>) -> Self {
        let (cap_tree, codex_entries, active_changes, archived_changes) = match duckspec_root {
            Some(root) => {
                let cap_tree = build_tree(&root.join("caps"), "caps");
                let codex_entries = build_tree_contents(&root.join("codex"), "codex");
                let active_changes = build_changes(&root.join("changes"), "changes");
                let archived_changes = build_changes(&root.join("archive"), "archive");
                (cap_tree, codex_entries, active_changes, archived_changes)
            }
            None => Default::default(),
        };

        let (validations, project_audit) = match duckspec_root {
            Some(ds) => run_audit(ds, Some(project_root)),
            None => (HashMap::new(), ProjectAudit::default()),
        };

        Self {
            project_root: Some(project_root.to_path_buf()),
            duckspec_root: duckspec_root.map(Path::to_path_buf),
            active_changes,
            archived_changes,
            cap_tree,
            codex_entries,
            validations,
            project_audit,
        }
    }

    pub fn reload(&mut self) {
        let old_validations = std::mem::take(&mut self.validations);
        let old_project_audit = std::mem::take(&mut self.project_audit);
        if let Some(project_root) = self.project_root.clone() {
            *self = Self::open(&project_root);
        } else {
            *self = Self::default();
        }
        // Preserve existing audit results — only refresh on explicit user action.
        self.validations = old_validations;
        self.project_audit = old_project_audit;
    }

    /// Re-run the full project audit.
    pub fn revalidate(&mut self) {
        if let Some(root) = &self.duckspec_root.clone() {
            let project_root = self.project_root.clone();
            let (validations, project_audit) = run_audit(root, project_root.as_deref());
            self.validations = validations;
            self.project_audit = project_audit;
        }
    }

    pub fn read_artifact(&self, id: &str) -> Option<String> {
        let root = self.duckspec_root.as_ref()?;
        fs::read_to_string(root.join(id)).ok()
    }
}

// ── Archive naming ───────────────────────────────────────────────────────────

/// Strip the `YYYY-MM-DD-NN-` prefix from an archived change folder name,
/// returning the base name. Returns `None` if the prefix doesn't match.
pub fn strip_archive_prefix(name: &str) -> Option<&str> {
    let bytes = name.as_bytes();
    if bytes.len() <= 14 {
        return None;
    }
    let is_digit = |i: usize| bytes[i].is_ascii_digit();
    let is_dash = |i: usize| bytes[i] == b'-';
    let ok = is_digit(0)
        && is_digit(1)
        && is_digit(2)
        && is_digit(3)
        && is_dash(4)
        && is_digit(5)
        && is_digit(6)
        && is_dash(7)
        && is_digit(8)
        && is_digit(9)
        && is_dash(10)
        && is_digit(11)
        && is_digit(12)
        && is_dash(13);
    if !ok {
        return None;
    }
    Some(&name[14..])
}

// ── Filesystem helpers ───────────────────────────────────────────────────────

fn read_sorted_dir(dir: &Path) -> Vec<fs::DirEntry> {
    let Ok(rd) = fs::read_dir(dir) else {
        return vec![];
    };
    let mut entries: Vec<_> = rd.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    entries
}

/// Build a tree of directories. Each directory node contains its markdown
/// files as leaf children, followed by subdirectory children (recursively).
fn build_tree(dir: &Path, id_prefix: &str) -> Vec<TreeNode> {
    let entries = read_sorted_dir(dir);
    let mut nodes = vec![];

    for entry in entries {
        if !is_dir(&entry) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let id = format!("{}/{}", id_prefix, name);
        let children = build_tree_contents(&entry.path(), &id);
        nodes.push(TreeNode {
            id,
            label: name,
            children,
        });
    }
    nodes
}

/// Build the children of a directory: markdown files first, then subdirs.
fn build_tree_contents(dir: &Path, id_prefix: &str) -> Vec<TreeNode> {
    let entries = read_sorted_dir(dir);
    let mut files = vec![];
    let mut dirs = vec![];

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let id = format!("{}/{}", id_prefix, name);

        if is_dir(&entry) {
            let children = build_tree_contents(&entry.path(), &id);
            dirs.push(TreeNode {
                id,
                label: name,
                children,
            });
        } else if name.ends_with(".md") {
            files.push(TreeNode {
                id,
                label: name,
                children: vec![],
            });
        }
    }

    files.extend(dirs);
    files
}

fn build_changes(dir: &Path, dir_prefix: &str) -> Vec<ChangeData> {
    let entries = read_sorted_dir(dir);
    let mut changes = vec![];

    for entry in entries {
        if !is_dir(&entry) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let prefix = format!("{}/{}", dir_prefix, name);
        let full = entry.path();

        let has_proposal = full.join("proposal.md").exists();
        let has_design = full.join("design.md").exists();
        let cap_tree = build_tree(&full.join("caps"), &format!("{}/caps", prefix));
        let steps = build_steps(&full.join("steps"), &prefix);

        changes.push(ChangeData {
            name,
            prefix,
            has_proposal,
            has_design,
            cap_tree,
            steps,
        });
    }
    changes
}

fn build_steps(dir: &Path, change_prefix: &str) -> Vec<StepInfo> {
    let entries = read_sorted_dir(dir);
    let mut steps = vec![];

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
            continue;
        }
        let stem = name.trim_end_matches(".md");
        if let Some((num_str, _)) = stem.split_once('-')
            && num_str.parse::<u32>().is_ok()
        {
            let completion = compute_step_completion(&entry.path());
            steps.push(StepInfo {
                id: format!("{}/steps/{}", change_prefix, name),
                label: name.clone(),
                completion,
            });
        }
    }
    steps
}

fn compute_step_completion(path: &Path) -> StepCompletion {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return StepCompletion::NoTasks,
    };
    let elements = duckpond::parse::parse_elements(&source);
    let step = match duckpond::parse::step::parse_step(&elements) {
        Ok(s) => s,
        Err(_) => return StepCompletion::NoTasks,
    };

    let total = step.tasks.len() + step.tasks.iter().map(|t| t.subtasks.len()).sum::<usize>();
    if total == 0 {
        return StepCompletion::NoTasks;
    }
    let done = step.tasks.iter().filter(|t| t.checked).count()
        + step
            .tasks
            .iter()
            .flat_map(|t| &t.subtasks)
            .filter(|s| s.checked)
            .count();
    if done == total {
        StepCompletion::Done
    } else {
        StepCompletion::Partial(done, total)
    }
}

fn is_dir(entry: &fs::DirEntry) -> bool {
    entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
}

// ── Audit ────────────────────────────────────────────────────────────────────

/// Run the full project audit and split results into per-change validations
/// and project-level audit findings. When `project_root` is unavailable, the
/// backlink scan is skipped (it falls back to the duckspec root's parent,
/// which is still usable for most layouts).
fn run_audit(
    duckspec_root: &Path,
    project_root: Option<&Path>,
) -> (HashMap<String, ChangeValidation>, ProjectAudit) {
    let project_root = project_root
        .map(Path::to_path_buf)
        .or_else(|| duckspec_root.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| duckspec_root.to_path_buf());

    let config = duckpond::config::Config::load(duckspec_root).unwrap_or_default();

    let report = match duckpond::audit::run_audit(
        duckspec_root,
        &project_root,
        &config,
        duckpond::audit::AuditScope::Full,
    ) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("audit failed: {e}");
            return (HashMap::new(), ProjectAudit::default());
        }
    };

    let mut validations: HashMap<String, ChangeValidation> = HashMap::new();
    for group in report.change_errors {
        let v = ChangeValidation {
            file_errors: group
                .file_errors
                .into_iter()
                .map(|(path, _source, errs)| {
                    (
                        path.display().to_string(),
                        errs.iter().map(|e| e.to_string()).collect(),
                    )
                })
                .collect(),
            change_errors: group.change_errors.iter().map(|e| e.to_string()).collect(),
        };
        if !v.is_empty() {
            validations.insert(group.change_name, v);
        }
    }

    // Surface unresolved step @spec refs as per-file errors so the sidebar
    // badge and inline error panel pick them up automatically.
    for r in &report.unresolved_step_refs {
        let step_id = r.step_file.display().to_string();
        let msg = format!("@spec does not resolve: {}", r.key.display());
        let entry = validations.entry(r.change_name.clone()).or_default();
        if let Some((_, msgs)) = entry.file_errors.iter_mut().find(|(p, _)| p == &step_id) {
            msgs.push(msg);
        } else {
            entry.file_errors.push((step_id, vec![msg]));
        }
    }

    let project_audit = ProjectAudit {
        artifact_errors: report
            .artifact_errors
            .into_iter()
            .map(|g| {
                (
                    g.relative_path.display().to_string(),
                    g.errors.iter().map(|e| e.to_string()).collect(),
                )
            })
            .collect(),
        unresolved_backlinks: report
            .unresolved_backlinks
            .into_iter()
            .map(|bl| BacklinkIssue {
                source_path: bl
                    .source_file
                    .strip_prefix(&project_root)
                    .unwrap_or(&bl.source_file)
                    .display()
                    .to_string(),
                line: bl.line,
                scenario_display: bl.key.display(),
            })
            .collect(),
        missing_backlink_scenarios: report
            .missing_backlink_scenarios
            .iter()
            .map(|k| k.display())
            .collect(),
        missing_step_coverage: {
            let mut out: HashMap<String, Vec<String>> = HashMap::new();
            for m in report.missing_step_coverage {
                out.entry(m.change_name)
                    .or_default()
                    .extend(m.missing.iter().map(|k| k.display()));
            }
            let mut v: Vec<_> = out.into_iter().collect();
            v.sort_by(|a, b| a.0.cmp(&b.0));
            v
        },
        unresolved_step_refs: report
            .unresolved_step_refs
            .into_iter()
            .map(|r| (r.change_name, r.key.display()))
            .collect(),
    };

    (validations, project_audit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_valid_archive_prefix() {
        assert_eq!(strip_archive_prefix("2026-04-20-01-foo"), Some("foo"));
        assert_eq!(
            strip_archive_prefix("2026-12-31-99-my-change"),
            Some("my-change")
        );
    }

    #[test]
    fn rejects_invalid_prefix() {
        assert_eq!(strip_archive_prefix("foo"), None);
        assert_eq!(strip_archive_prefix("2026-04-20-foo"), None);
        assert_eq!(strip_archive_prefix("26-04-20-01-foo"), None);
        assert_eq!(strip_archive_prefix("2026-4-20-01-foo"), None);
        assert_eq!(strip_archive_prefix("2026-04-20-01-"), None);
    }
}
