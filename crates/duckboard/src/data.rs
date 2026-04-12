//! Project data model and filesystem loader.

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

#[derive(Debug, Clone)]
pub struct StepInfo {
    pub id: String,
    pub label: String,
    pub number: u32,
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

        Self {
            project_root: find_repo_root(),
            duckspec_root: Some(root.to_path_buf()),
            active_changes,
            archived_changes,
            cap_tree,
            codex_entries,
            cap_count,
            codex_count,
        }
    }

    pub fn reload(&mut self) {
        if let Some(root) = self.duckspec_root.clone() {
            *self = Self::load_from(&root);
        } else {
            *self = Self::load();
        }
    }

    pub fn read_artifact(&self, id: &str) -> Option<String> {
        let root = self.duckspec_root.as_ref()?;
        fs::read_to_string(root.join(id)).ok()
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
        if let Some((num_str, slug)) = stem.split_once('-') {
            if let Ok(number) = num_str.parse::<u32>() {
                steps.push(StepInfo {
                    id: format!("{}/steps/{}", change_prefix, name),
                    label: slug.replace('-', " "),
                    number,
                });
            }
        }
    }
    steps
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
