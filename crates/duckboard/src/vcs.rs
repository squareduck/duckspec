//! Version control integration via gix.
//!
//! Provides changed file detection and unified diff generation for git
//! repositories (including jj-managed repos, which always have a git backend).

use std::path::{Path, PathBuf};

// ── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
}

#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub path: PathBuf,
    pub status: FileStatus,
}

#[derive(Debug, Clone)]
pub struct DiffData {
    pub path: PathBuf,
    pub status: FileStatus,
    pub hunks: Vec<Hunk>,
    /// Old (HEAD) file content, kept for syntax highlighting.
    pub old_content: String,
    /// New (working tree) file content, kept for syntax highlighting.
    pub new_content: String,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Added,
    Removed,
}

// ── Changed files ───────────────────────────────────────────────────────────

/// List files that differ between HEAD and the working tree.
pub fn changed_files(repo_root: &Path) -> Vec<ChangedFile> {
    let repo = match gix::open(repo_root) {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!("failed to open repo at {}: {e}", repo_root.display());
            return vec![];
        }
    };

    let status = match repo.status(gix::progress::Discard) {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!("failed to get status: {e}");
            return vec![];
        }
    };

    let iter = match status.into_iter(std::iter::empty::<gix::bstr::BString>()) {
        Ok(i) => i,
        Err(e) => {
            tracing::debug!("failed to iterate status: {e}");
            return vec![];
        }
    };

    let mut files = Vec::new();
    for item in iter {
        let item = match item {
            Ok(i) => i,
            Err(_) => continue,
        };

        use gix::status::Item;

        let (rela_path, file_status) = match &item {
            Item::IndexWorktree(iw) => {
                use gix::status::index_worktree::iter::Summary;
                let summary = match iw.summary() {
                    Some(s) => s,
                    None => continue,
                };
                let fs = match summary {
                    Summary::Modified | Summary::TypeChange => FileStatus::Modified,
                    Summary::Added | Summary::IntentToAdd => FileStatus::Added,
                    Summary::Removed => FileStatus::Deleted,
                    Summary::Renamed | Summary::Copied => FileStatus::Modified,
                    Summary::Conflict => FileStatus::Modified,
                };
                (iw.rela_path().to_string(), fs)
            }
            Item::TreeIndex(change) => {
                use gix::diff::index::Change;
                let rela = change.fields().0;
                let fs = match change {
                    Change::Addition { .. } => FileStatus::Added,
                    Change::Deletion { .. } => FileStatus::Deleted,
                    Change::Modification { .. } => FileStatus::Modified,
                    Change::Rewrite { .. } => FileStatus::Modified,
                };
                (rela.to_string(), fs)
            }
        };

        files.push(ChangedFile {
            path: PathBuf::from(rela_path),
            status: file_status,
        });
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    files.dedup_by(|a, b| a.path == b.path);
    files
}

// ── File diff ───────────────────────────────────────────────────────────────

/// Generate a unified diff for a single file between HEAD and the working tree.
pub fn file_diff(repo_root: &Path, rel_path: &Path) -> Option<DiffData> {
    let repo = gix::open(repo_root).ok()?;
    let status = find_file_status(rel_path, &changed_files(repo_root))?;

    let old_content = read_head_blob(&repo, rel_path).unwrap_or_default();
    let new_content = match status {
        FileStatus::Deleted => String::new(),
        _ => std::fs::read_to_string(repo_root.join(rel_path)).unwrap_or_default(),
    };

    let diff = similar::TextDiff::from_lines(&old_content, &new_content);
    let hunks = build_hunks(&diff);

    Some(DiffData {
        path: rel_path.to_path_buf(),
        status,
        hunks,
        old_content,
        new_content,
    })
}

fn find_file_status(rel_path: &Path, files: &[ChangedFile]) -> Option<FileStatus> {
    files.iter().find(|f| f.path == rel_path).map(|f| f.status)
}

fn read_head_blob(repo: &gix::Repository, rel_path: &Path) -> Option<String> {
    let head = repo.head_commit().ok()?;
    let tree = head.tree().ok()?;
    let entry = tree.lookup_entry_by_path(rel_path).ok()??;
    let object = entry.object().ok()?;
    let data = object.detach().data;
    String::from_utf8(data).ok()
}

fn build_hunks<'a>(diff: &similar::TextDiff<'a, 'a, 'a, str>) -> Vec<Hunk> {
    diff.unified_diff()
        .context_radius(3)
        .iter_hunks()
        .map(|hunk| {
            let header = hunk.header().to_string();

            let lines: Vec<DiffLine> = hunk
                .iter_changes()
                .map(|change| {
                    use similar::ChangeTag;
                    let (kind, old_no, new_no) = match change.tag() {
                        ChangeTag::Equal => (
                            LineKind::Context,
                            change.old_index().map(|i| i as u32 + 1),
                            change.new_index().map(|i| i as u32 + 1),
                        ),
                        ChangeTag::Delete => (
                            LineKind::Removed,
                            change.old_index().map(|i| i as u32 + 1),
                            None,
                        ),
                        ChangeTag::Insert => (
                            LineKind::Added,
                            None,
                            change.new_index().map(|i| i as u32 + 1),
                        ),
                    };
                    DiffLine {
                        kind,
                        old_lineno: old_no,
                        new_lineno: new_no,
                        text: change.to_string_lossy().to_string(),
                    }
                })
                .collect();

            Hunk { header, lines }
        })
        .collect()
}
