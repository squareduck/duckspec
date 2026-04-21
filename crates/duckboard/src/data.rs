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
    pub cap_count: usize,
    pub codex_count: usize,
    /// Validation results per change name, populated on reload.
    pub validations: HashMap<String, ChangeValidation>,
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
        self.file_errors.iter().map(|(_, errs)| errs.len()).sum::<usize>()
            + self.change_errors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.file_errors.is_empty() && self.change_errors.is_empty()
    }
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
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

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
    pub fn load() -> Self {
        match find_duckspec_root() {
            Some(root) => Self::load_from(&root),
            None => Self::default(),
        }
    }

    fn load_from(root: &Path) -> Self {
        let cap_tree = build_tree(&root.join("caps"), "caps");
        let cap_count = count_leaf_caps(&cap_tree);
        let codex_entries = build_file_list(&root.join("codex"), "codex");
        let codex_count = codex_entries.len();
        let active_changes = build_changes(&root.join("changes"), "changes");
        let archived_changes = build_changes(&root.join("archive"), "archive");
        let validations = run_validations(root, &active_changes);

        Self {
            project_root: find_repo_root(),
            duckspec_root: Some(root.to_path_buf()),
            active_changes,
            archived_changes,
            cap_tree,
            codex_entries,
            cap_count,
            codex_count,
            validations,
        }
    }

    pub fn reload(&mut self) {
        let old_validations = std::mem::take(&mut self.validations);
        if let Some(root) = self.duckspec_root.clone() {
            *self = Self::load_from(&root);
        } else {
            *self = Self::load();
        }
        // Preserve existing validations — only refresh on explicit user action.
        self.validations = old_validations;
    }

    /// Re-run validation for all active changes.
    pub fn revalidate(&mut self) {
        if let Some(root) = &self.duckspec_root {
            self.validations = run_validations(root, &self.active_changes);
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
    let ok = is_digit(0) && is_digit(1) && is_digit(2) && is_digit(3)
        && is_dash(4)
        && is_digit(5) && is_digit(6)
        && is_dash(7)
        && is_digit(8) && is_digit(9)
        && is_dash(10)
        && is_digit(11) && is_digit(12)
        && is_dash(13);
    if !ok {
        return None;
    }
    Some(&name[14..])
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

// ── Filesystem helpers ───────────────────────────────────────────────────────

/// Find the repository root by walking up from cwd looking for `.git` or `.jj`.
fn find_repo_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join(".git").exists() || dir.join(".jj").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn find_duckspec_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("duckspec");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

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

/// Flat list of markdown files in a directory (non-recursive).
fn build_file_list(dir: &Path, id_prefix: &str) -> Vec<TreeNode> {
    let entries = read_sorted_dir(dir);
    let mut nodes = vec![];

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".md") && !is_dir(&entry) {
            let stem = name.trim_end_matches(".md").to_string();
            nodes.push(TreeNode {
                id: format!("{}/{}", id_prefix, name),
                label: stem,
                children: vec![],
            });
        }
    }
    nodes
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
            && num_str.parse::<u32>().is_ok() {
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

    let total = step.tasks.len()
        + step.tasks.iter().map(|t| t.subtasks.len()).sum::<usize>();
    if total == 0 {
        return StepCompletion::NoTasks;
    }
    let done = step.tasks.iter().filter(|t| t.checked).count()
        + step.tasks.iter().flat_map(|t| &t.subtasks).filter(|s| s.checked).count();
    if done == total {
        StepCompletion::Done
    } else {
        StepCompletion::Partial(done, total)
    }
}

/// Count capabilities (directories that have a spec.md child).
fn count_leaf_caps(nodes: &[TreeNode]) -> usize {
    let mut count = 0;
    for node in nodes {
        if node
            .children
            .iter()
            .any(|c| c.label == "spec.md" || c.label == "doc.md")
        {
            count += 1;
        }
        let subdirs: Vec<_> = node.children.iter().filter(|c| !c.is_leaf()).collect();
        count += count_leaf_caps(&subdirs.iter().map(|n| (*n).clone()).collect::<Vec<_>>());
    }
    count
}

fn is_dir(entry: &fs::DirEntry) -> bool {
    entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
}

// ── Validation ──────────────────────────────────────────────────────────────

/// Run `check_change` for all active changes and collect results.
fn run_validations(
    duckspec_root: &Path,
    active_changes: &[ChangeData],
) -> HashMap<String, ChangeValidation> {
    let state = build_duckspec_state(duckspec_root);
    let mut results = HashMap::new();

    for change in active_changes {
        let files = load_change_files(duckspec_root, &change.name);
        let check = duckpond::check::check_change(&change.name, &files, &state);

        let validation = ChangeValidation {
            file_errors: check
                .file_errors
                .into_iter()
                .map(|(path, errs)| {
                    (
                        path.display().to_string(),
                        errs.iter().map(|e| e.to_string()).collect(),
                    )
                })
                .collect(),
            change_errors: check
                .change_errors
                .iter()
                .map(|e| e.to_string())
                .collect(),
        };

        if !validation.is_empty() {
            results.insert(change.name.clone(), validation);
        }
    }

    results
}

/// Build `DuckspecState` by scanning `caps/` for existing spec.md and doc.md files.
fn build_duckspec_state(duckspec_root: &Path) -> duckpond::check::DuckspecState {
    let caps_dir = duckspec_root.join("caps");
    let mut cap_spec_paths = std::collections::HashSet::new();
    let mut cap_doc_paths = std::collections::HashSet::new();

    if caps_dir.is_dir() {
        scan_caps_for_state(&caps_dir, &caps_dir, &mut cap_spec_paths, &mut cap_doc_paths);
    }

    duckpond::check::DuckspecState {
        cap_spec_paths,
        cap_doc_paths,
    }
}

/// Recursively scan `caps/` for spec.md and doc.md, collecting capability paths.
fn scan_caps_for_state(
    dir: &Path,
    caps_root: &Path,
    spec_paths: &mut std::collections::HashSet<PathBuf>,
    doc_paths: &mut std::collections::HashSet<PathBuf>,
) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_caps_for_state(&path, caps_root, spec_paths, doc_paths);
        } else if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            if let Some(cap_path) = path.parent().and_then(|p| p.strip_prefix(caps_root).ok()) {
                match filename {
                    "spec.md" => { spec_paths.insert(cap_path.to_path_buf()); }
                    "doc.md" => { doc_paths.insert(cap_path.to_path_buf()); }
                    _ => {}
                }
            }
        }
    }
}

/// Load all classifiable markdown files in a change directory as `LoadedFile`s.
fn load_change_files(duckspec_root: &Path, change_name: &str) -> Vec<duckpond::check::LoadedFile> {
    let change_dir = duckspec_root.join("changes").join(change_name);
    let mut files = Vec::new();
    collect_md_files_recursive(&change_dir, duckspec_root, &mut files);
    files
}

fn collect_md_files_recursive(
    dir: &Path,
    duckspec_root: &Path,
    out: &mut Vec<duckpond::check::LoadedFile>,
) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files_recursive(&path, duckspec_root, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let Ok(relative) = path.strip_prefix(duckspec_root) else { continue };
            let Some(kind) = duckpond::layout::classify(relative) else { continue };
            let Ok(content) = fs::read_to_string(&path) else { continue };
            out.push(duckpond::check::LoadedFile {
                relative_path: relative.to_path_buf(),
                kind,
                content,
            });
        }
    }
}
