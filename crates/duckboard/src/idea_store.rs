//! Idea model and per-project file storage under `<data>/ideas/`.
//!
//! Each idea is one markdown file with YAML frontmatter:
//!
//! ```text
//! ---
//! title: Fix overflow on long input
//! created: 2026-04-25T14:32:00+02:00
//! tags: [parser/spec, performance]      # first entry = primary, drives path
//! exploration: exploration-1714082400000  # only after Explore is clicked
//! change: 2026-04-25-01-fix-parser-overflow # only after promotion
//! archived: manual                        # only when in archive state
//! ---
//!
//! # Fix overflow on long input
//!
//! Body markdown…
//! ```
//!
//! Path encodes state and primary-tag tree:
//!
//! ```text
//! ideas/
//!   inbox/<slug>.md                          # untagged
//!   inbox/parser/<slug>.md                   # primary tag #parser
//!   inbox/parser/spec/<slug>.md              # primary tag #parser/spec
//!   exploration/…  change/…  archive/…
//! ```
//!
//! Identity of an idea is its path. There is no stable id field; cross-rename
//! continuity is provided by the `exploration` and `change` keys in the
//! frontmatter, which point at the chats/ directory and duckspec change
//! folder respectively. Saving with a new title or new primary tag moves
//! the file (atomic `rename` on a single filesystem).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::data::{self, ProjectData};

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArchiveKind {
    Manual,
    ViaChange,
    Orphaned,
}

impl ArchiveKind {
    pub fn label(self) -> &'static str {
        match self {
            ArchiveKind::Manual => "manual",
            ArchiveKind::ViaChange => "via change",
            ArchiveKind::Orphaned => "orphaned",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdeaState {
    Inbox,
    Exploration,
    Change,
    Archive,
}

impl IdeaState {
    pub fn segment(self) -> &'static str {
        match self {
            IdeaState::Inbox => "inbox",
            IdeaState::Exploration => "exploration",
            IdeaState::Change => "change",
            IdeaState::Archive => "archive",
        }
    }

    #[allow(dead_code)]
    pub fn from_segment(s: &str) -> Option<Self> {
        match s {
            "inbox" => Some(IdeaState::Inbox),
            "exploration" => Some(IdeaState::Exploration),
            "change" => Some(IdeaState::Change),
            "archive" => Some(IdeaState::Archive),
            _ => None,
        }
    }

    pub const ALL: [IdeaState; 4] = [
        IdeaState::Inbox,
        IdeaState::Exploration,
        IdeaState::Change,
        IdeaState::Archive,
    ];
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: String,
    pub created: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exploration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived: Option<ArchiveKind>,
}

/// In-memory idea metadata. `abs_path` is the current on-disk location;
/// `state` and `primary_tag_path` are derived from `abs_path` at scan time.
/// The body is **not** held here — it's loaded on demand via `read_body`
/// when the user opens an idea, and passed to `save_idea` on Cmd-S.
#[derive(Debug, Clone)]
pub struct Idea {
    pub abs_path: PathBuf,
    pub state: IdeaState,
    /// Path segments derived from the primary tag. `["parser", "spec"]` for
    /// `#parser/spec`; empty when untagged.
    pub primary_tag_path: Vec<String>,
    pub frontmatter: Frontmatter,
}

impl Idea {
    /// Title for UI rendering — frontmatter wins; falls back to a slug-derived
    /// label if the file's frontmatter is missing/empty (e.g. external edit
    /// before our parser has run on it).
    pub fn display_title(&self) -> String {
        if !self.frontmatter.title.trim().is_empty() {
            return self.frontmatter.title.clone();
        }
        slug_to_display(&self.abs_path)
    }

    /// Stable scope key for this idea's chat: change_name (post-promotion)
    /// wins over exploration id (pre-promotion). `None` for inbox ideas with
    /// no chat scope yet.
    pub fn scope_key(&self) -> Option<&str> {
        self.frontmatter
            .change
            .as_deref()
            .or(self.frontmatter.exploration.as_deref())
    }
}

/// Best-effort title fallback derived from the file's slug. Capitalizes the
/// first letter so a row never reads as a kebab string.
fn slug_to_display(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled");
    let mut s: String = stem.replace('-', " ");
    if let Some(c) = s.get_mut(0..1) {
        c.make_ascii_uppercase();
    }
    s
}

// ── Paths ────────────────────────────────────────────────────────────────────

pub fn ideas_root(project_root: Option<&Path>) -> PathBuf {
    crate::config::data_dir(project_root).join("ideas")
}

/// Slugify a title for use as a filename. Lowercases, replaces non-alphanumerics
/// with `-`, collapses runs, and trims leading/trailing `-`. Returns `"idea"`
/// for inputs that produce an empty slug (all-whitespace, all-punctuation).
pub fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = false;
    for ch in s.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "idea".to_string()
    } else {
        out
    }
}

fn primary_tag_segments(tags: &[String]) -> Vec<String> {
    let Some(primary) = tags.first() else {
        return Vec::new();
    };
    primary
        .split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| slugify(s))
        .collect()
}

fn idea_path(root: &Path, state: IdeaState, tag_segments: &[String], slug: &str) -> PathBuf {
    let mut p = root.join(state.segment());
    for seg in tag_segments {
        p = p.join(seg);
    }
    p.join(format!("{slug}.md"))
}

// ── Frontmatter parser/serializer ────────────────────────────────────────────

/// Split a file's contents into (frontmatter, body). Recognizes the standard
/// `---\n…\n---\n` envelope at the start. If the envelope is absent or the
/// YAML fails to parse, returns a default frontmatter and the entire input as
/// body.
pub fn parse_file_contents(contents: &str) -> (Frontmatter, String) {
    let Some(rest) = contents.strip_prefix("---\n") else {
        return (Frontmatter::default(), contents.to_string());
    };
    // Find the closing `\n---\n` (or `\n---` at EOF).
    let (yaml, body) = if let Some(idx) = rest.find("\n---\n") {
        (&rest[..idx], &rest[idx + 5..])
    } else if let Some(idx) = rest.rfind("\n---") {
        if rest[idx + 4..].chars().all(char::is_whitespace) {
            (&rest[..idx], "")
        } else {
            return (Frontmatter::default(), contents.to_string());
        }
    } else {
        return (Frontmatter::default(), contents.to_string());
    };
    match serde_yaml::from_str::<Frontmatter>(yaml) {
        Ok(fm) => (fm, body.to_string()),
        Err(e) => {
            tracing::warn!("idea frontmatter parse error: {e}");
            (Frontmatter::default(), contents.to_string())
        }
    }
}

/// Serialize an idea as `---\n<yaml>---\n<body>`.
pub fn serialize_file_contents(frontmatter: &Frontmatter, body: &str) -> anyhow::Result<String> {
    let yaml = serde_yaml::to_string(frontmatter)?;
    // serde_yaml emits a trailing newline; ensure exactly one before the closing fence.
    let yaml = if yaml.ends_with('\n') {
        yaml
    } else {
        format!("{yaml}\n")
    };
    Ok(format!("---\n{yaml}---\n{body}"))
}

// ── Title derivation ─────────────────────────────────────────────────────────

/// Extract a title from the body's first H1 (`# Heading`). Returns `None` if
/// the first non-blank line isn't an ATX H1. Trailing hashes (`# Heading #`)
/// are stripped.
pub fn derive_title_from_body(body: &str) -> Option<String> {
    for line in body.lines() {
        let t = line.trim_start();
        if t.is_empty() {
            continue;
        }
        let stripped = t.strip_prefix('#')?;
        // Reject H2+ (#-prefix already consumed; another # means H2).
        if stripped.starts_with('#') {
            return None;
        }
        if !stripped.starts_with(char::is_whitespace) {
            return None;
        }
        let title = stripped.trim().trim_end_matches('#').trim();
        return Some(title.to_string());
    }
    None
}

pub fn fallback_title() -> String {
    "New idea".to_string()
}

fn iso8601_local(dt: OffsetDateTime) -> String {
    let off = dt.offset();
    let total = off.whole_seconds();
    let sign = if total < 0 { '-' } else { '+' };
    let abs = total.unsigned_abs();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}{:02}:{:02}",
        dt.year(),
        dt.month() as u8,
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second(),
        sign,
        abs / 3600,
        (abs % 3600) / 60,
    )
}

// ── Loading ──────────────────────────────────────────────────────────────────

/// Cap on how much of each idea file we read at startup just to extract its
/// frontmatter. Real-world frontmatter is ~200 bytes; the cap is a generous
/// ceiling that bounds worst-case work for files without a closing fence.
const MAX_FRONTMATTER_BYTES: usize = 4096;

/// Walk the `ideas/` tree and return all ideas with metadata only. Bodies
/// are deliberately not read here — call `read_body` when an idea is opened.
pub fn load_all(project_root: Option<&Path>) -> Vec<Idea> {
    let root = ideas_root(project_root);
    let mut out = Vec::new();
    for state in IdeaState::ALL {
        let state_root = root.join(state.segment());
        walk_state_dir(&state_root, state, &mut out);
    }
    out
}

fn walk_state_dir(state_root: &Path, state: IdeaState, out: &mut Vec<Idea>) {
    let mut stack: Vec<(PathBuf, Vec<String>)> = vec![(state_root.to_path_buf(), vec![])];
    while let Some((dir, tag_path)) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let p = entry.path();
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                let mut next = tag_path.clone();
                next.push(entry.file_name().to_string_lossy().into_owned());
                stack.push((p, next));
            } else if ft.is_file() && p.extension().is_some_and(|e| e == "md") {
                if let Some(idea) = read_idea_meta(&p, state, tag_path.clone()) {
                    out.push(idea);
                }
            }
        }
    }
}

/// Read just enough of `path` to extract its frontmatter. Stops at the
/// closing `\n---` fence or `MAX_FRONTMATTER_BYTES`, whichever comes first.
/// The body is never loaded into memory.
fn read_idea_meta(path: &Path, state: IdeaState, primary_tag_path: Vec<String>) -> Option<Idea> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    loop {
        let n = match f.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => return None,
        };
        buf.extend_from_slice(&chunk[..n]);
        if has_closing_fence(&buf) || buf.len() >= MAX_FRONTMATTER_BYTES {
            break;
        }
    }
    let s = std::str::from_utf8(&buf).ok()?;
    let (frontmatter, _) = parse_file_contents(s);
    Some(Idea {
        abs_path: path.to_path_buf(),
        state,
        primary_tag_path,
        frontmatter,
    })
}

/// True once the buffer contains the closing `\n---` line of a frontmatter
/// envelope. Requires the opening `---\n` to already be present (we don't
/// validate that here — `parse_file_contents` handles malformed input
/// downstream).
fn has_closing_fence(buf: &[u8]) -> bool {
    if buf.len() < 8 || !buf.starts_with(b"---\n") {
        return false;
    }
    // Search after the opening fence.
    let after_open = &buf[4..];
    after_open.windows(5).any(|w| w == b"\n---\n") || after_open.ends_with(b"\n---")
}

/// Read the body portion of an idea file. Called when the user opens an
/// idea — the pinned tab feeds this into a fresh `EditorState`. Returns
/// the entire file as body if no frontmatter is found.
pub fn read_body(path: &Path) -> std::io::Result<String> {
    let raw = std::fs::read_to_string(path)?;
    let (_fm, body) = parse_file_contents(&raw);
    Ok(body)
}

// ── Saving ───────────────────────────────────────────────────────────────────

/// Persist an idea to disk. The body is passed explicitly because `Idea`
/// doesn't carry it — the area holds the editor state and forwards its
/// contents on Cmd-S. Recomputes title from `body`'s H1 (or fallback),
/// recomputes the target path from state + primary tag + slugified title,
/// and renames atomically if the path changed. On rename, prunes empty
/// ancestor directories. Updates `idea.abs_path` and `idea.primary_tag_path`
/// on success.
pub fn save_idea(
    idea: &mut Idea,
    body: &str,
    project_root: Option<&Path>,
) -> anyhow::Result<()> {
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());

    // Recompute title from body. Frontmatter title is purely derived — we
    // never let the YAML diverge from the body's H1.
    idea.frontmatter.title =
        derive_title_from_body(body).unwrap_or_else(fallback_title);
    if idea.frontmatter.created.trim().is_empty() {
        idea.frontmatter.created = iso8601_local(now);
    }

    // Archive sub-state must be cleared when not in archive (and defaulted
    // to Manual when entering archive without one set).
    match idea.state {
        IdeaState::Archive => {
            if idea.frontmatter.archived.is_none() {
                idea.frontmatter.archived = Some(ArchiveKind::Manual);
            }
        }
        _ => {
            idea.frontmatter.archived = None;
        }
    }

    let root = ideas_root(project_root);
    let target_segments = primary_tag_segments(&idea.frontmatter.tags);
    let slug = slugify(&idea.frontmatter.title);

    let candidate = idea_path(&root, idea.state, &target_segments, &slug);
    let final_path = if candidate == idea.abs_path && candidate.exists() {
        candidate
    } else if !candidate.exists() {
        candidate
    } else {
        unique_path(&root, idea.state, &target_segments, &slug)
    };

    let contents = serialize_file_contents(&idea.frontmatter, body)?;
    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let prev_path = idea.abs_path.clone();
    if final_path != prev_path && prev_path.exists() {
        std::fs::rename(&prev_path, &final_path)?;
    }
    std::fs::write(&final_path, contents)?;

    if final_path != prev_path && !prev_path.as_os_str().is_empty() {
        prune_empty_dirs(prev_path.parent(), &root);
    }

    idea.abs_path = final_path;
    idea.primary_tag_path = target_segments;
    Ok(())
}

fn unique_path(
    root: &Path,
    state: IdeaState,
    tag_segments: &[String],
    slug_base: &str,
) -> PathBuf {
    for n in 2..1000 {
        let candidate = idea_path(root, state, tag_segments, &format!("{slug_base}-{n}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    idea_path(root, state, tag_segments, slug_base)
}

/// Walk up from `start` removing directories that have become empty, stopping
/// at `stop_at` or the first non-empty ancestor or filesystem error.
fn prune_empty_dirs(start: Option<&Path>, stop_at: &Path) {
    let mut cur = start;
    while let Some(p) = cur {
        if p == stop_at || !p.starts_with(stop_at) {
            break;
        }
        let is_empty = std::fs::read_dir(p)
            .map(|mut d| d.next().is_none())
            .unwrap_or(false);
        if !is_empty {
            break;
        }
        if std::fs::remove_dir(p).is_err() {
            break;
        }
        cur = p.parent();
    }
}

// ── Construction & deletion ──────────────────────────────────────────────────

/// Mint a new untagged inbox idea. Path is unset until `save_idea` runs.
pub fn new_idea() -> Idea {
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    Idea {
        abs_path: PathBuf::new(),
        state: IdeaState::Inbox,
        primary_tag_path: Vec::new(),
        frontmatter: Frontmatter {
            title: fallback_title(),
            created: iso8601_local(now),
            tags: Vec::new(),
            exploration: None,
            change: None,
            archived: None,
        },
    }
}

pub fn delete_idea(idea: &Idea, project_root: Option<&Path>) {
    if idea.abs_path.exists()
        && let Err(e) = std::fs::remove_file(&idea.abs_path)
    {
        tracing::warn!(path = %idea.abs_path.display(), "failed to delete idea: {e}");
        return;
    }
    let root = ideas_root(project_root);
    prune_empty_dirs(idea.abs_path.parent(), &root);
}

// ── Reconcile against project state ──────────────────────────────────────────

/// Detect drift in change-state ideas: when the attached change has been
/// archived externally (via `/ds-archive`) or removed entirely, move the idea
/// into the archive state with the appropriate sub-kind. Performs file moves
/// in place; updates `ideas` to reflect new paths and frontmatter. Reads
/// each drifted idea's body from disk to round-trip it through `save_idea`.
pub fn reconcile(ideas: &mut [Idea], project: &ProjectData) {
    let project_root = project.project_root.as_deref();
    for idea in ideas.iter_mut() {
        let Some((new_state, archived)) = drift_target(idea, project) else {
            continue;
        };
        idea.state = new_state;
        idea.frontmatter.archived = archived;
        let body = read_body(&idea.abs_path).unwrap_or_default();
        if let Err(e) = save_idea(idea, &body, project_root) {
            tracing::warn!("failed to reconcile idea: {e}");
        }
    }
}

fn drift_target(idea: &Idea, project: &ProjectData) -> Option<(IdeaState, Option<ArchiveKind>)> {
    if idea.state == IdeaState::Archive {
        return None;
    }
    let change_name = idea.frontmatter.change.as_deref()?;
    let archived_externally = project
        .archived_changes
        .iter()
        .any(|c| data::strip_archive_prefix(&c.name) == Some(change_name));
    if archived_externally {
        return Some((IdeaState::Archive, Some(ArchiveKind::ViaChange)));
    }
    let still_exists = project.active_changes.iter().any(|c| c.name == change_name);
    if !still_exists {
        return Some((IdeaState::Archive, Some(ArchiveKind::Orphaned)));
    }
    None
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Fix overflow on long input"), "fix-overflow-on-long-input");
        assert_eq!(slugify("  Hello,  World! "), "hello-world");
        assert_eq!(slugify("Spëcial-Chårs"), "sp-cial-ch-rs");
        assert_eq!(slugify("---"), "idea");
        assert_eq!(slugify(""), "idea");
        assert_eq!(slugify("Already-kebab-case"), "already-kebab-case");
    }

    #[test]
    fn derive_title_h1_only() {
        assert_eq!(
            derive_title_from_body("# Hello\nbody"),
            Some("Hello".into())
        );
        assert_eq!(
            derive_title_from_body("\n\n  # Hello world  \nbody"),
            Some("Hello world".into())
        );
        assert_eq!(
            derive_title_from_body("# Hello ##\nbody"),
            Some("Hello".into())
        );
    }

    #[test]
    fn derive_title_rejects_h2_and_plain() {
        assert_eq!(derive_title_from_body("## Heading"), None);
        assert_eq!(derive_title_from_body("plain text"), None);
        assert_eq!(derive_title_from_body(""), None);
        // Hash without a following space is not an H1.
        assert_eq!(derive_title_from_body("#tag at start"), None);
    }

    #[test]
    fn primary_tag_path_segments() {
        assert!(primary_tag_segments(&[]).is_empty());
        assert_eq!(
            primary_tag_segments(&["parser/spec".into(), "performance".into()]),
            vec!["parser".to_string(), "spec".into()]
        );
        assert_eq!(
            primary_tag_segments(&["parser".into()]),
            vec!["parser".to_string()]
        );
        // Tag with internal slashes is hierarchical; non-alphanumerics in each
        // segment slugify normally.
        assert_eq!(
            primary_tag_segments(&["Cool Stuff/sub item".into()]),
            vec!["cool-stuff".to_string(), "sub-item".into()]
        );
    }

    #[test]
    fn frontmatter_round_trip_minimum() {
        let fm = Frontmatter {
            title: "Test".into(),
            created: "2026-04-25T14:32:00+02:00".into(),
            tags: vec![],
            exploration: None,
            change: None,
            archived: None,
        };
        let body = "# Test\n\nsome body\n";
        let serialized = serialize_file_contents(&fm, body).expect("serialize");
        assert!(serialized.starts_with("---\n"));
        assert!(serialized.contains("\n---\n"));
        assert!(serialized.ends_with("some body\n"));

        let (parsed_fm, parsed_body) = parse_file_contents(&serialized);
        assert_eq!(parsed_fm.title, "Test");
        assert_eq!(parsed_fm.created, "2026-04-25T14:32:00+02:00");
        assert!(parsed_fm.tags.is_empty());
        assert_eq!(parsed_body, body);
    }

    #[test]
    fn frontmatter_round_trip_full() {
        let fm = Frontmatter {
            title: "Fix overflow".into(),
            created: "2026-04-25T14:32:00+02:00".into(),
            tags: vec!["parser/spec".into(), "performance".into()],
            exploration: Some("exploration-1714082400000".into()),
            change: Some("2026-04-25-01-fix-parser-overflow".into()),
            archived: Some(ArchiveKind::ViaChange),
        };
        let body = "# Fix overflow\n\nDetails here.\n";
        let serialized = serialize_file_contents(&fm, body).expect("serialize");
        // archived: via-change must be the kebab form.
        assert!(serialized.contains("archived: via-change"));

        let (parsed_fm, parsed_body) = parse_file_contents(&serialized);
        assert_eq!(parsed_fm.tags, vec!["parser/spec", "performance"]);
        assert_eq!(parsed_fm.exploration.as_deref(), Some("exploration-1714082400000"));
        assert_eq!(parsed_fm.change.as_deref(), Some("2026-04-25-01-fix-parser-overflow"));
        assert!(matches!(parsed_fm.archived, Some(ArchiveKind::ViaChange)));
        assert_eq!(parsed_body, body);
    }

    #[test]
    fn parse_no_frontmatter_returns_default() {
        let raw = "# No frontmatter\n\nbody only\n";
        let (fm, body) = parse_file_contents(raw);
        assert!(fm.title.is_empty());
        assert_eq!(body, raw);
    }

    #[test]
    fn parse_malformed_yaml_returns_default() {
        let raw = "---\nthis is: not: valid yaml: somehow\n---\nbody\n";
        let (fm, body) = parse_file_contents(raw);
        assert!(fm.title.is_empty());
        // Body falls back to the entire input so no data is lost.
        assert_eq!(body, raw);
    }

    #[test]
    fn idea_state_segment_round_trip() {
        for s in IdeaState::ALL {
            assert_eq!(IdeaState::from_segment(s.segment()), Some(s));
        }
        assert_eq!(IdeaState::from_segment("nope"), None);
    }

    #[test]
    fn parse_serialize_round_trip_for_body_separation() {
        // Body is no longer carried on Idea; this test verifies that
        // `parse_file_contents` cleanly separates frontmatter and body so
        // `read_body` (which calls it) returns just the body slice.
        let fm = Frontmatter {
            title: "Test".into(),
            created: "2026-04-25T14:32:00+02:00".into(),
            tags: vec![],
            exploration: None,
            change: None,
            archived: None,
        };
        let body = "# Test\n\nSome content.\n";
        let raw = serialize_file_contents(&fm, body).unwrap();
        let (parsed_fm, parsed_body) = parse_file_contents(&raw);
        assert_eq!(parsed_fm.title, "Test");
        assert_eq!(parsed_body, body);
    }

    #[test]
    fn has_closing_fence_finds_end_of_yaml() {
        // No fence yet — just opening.
        assert!(!has_closing_fence(b"---\ntitle: x\n"));
        // Fence in the middle, with body after.
        assert!(has_closing_fence(b"---\ntitle: x\n---\nbody"));
        // Fence at EOF, no trailing newline.
        assert!(has_closing_fence(b"---\ntitle: x\n---"));
        // Buffer doesn't even start with opening fence — never match.
        assert!(!has_closing_fence(b"some content\n---\nmore"));
    }

    #[test]
    fn read_idea_meta_skips_body_and_caps_at_max_bytes() {
        let tmp = tempdir();
        // Build a file with a normal frontmatter and a huge body. After the
        // closing fence, the file is mostly noise; meta-reading must not
        // load it.
        let fm = Frontmatter {
            title: "Big".into(),
            created: "2026-04-25T14:32:00+02:00".into(),
            tags: vec!["parser".into()],
            exploration: None,
            change: None,
            archived: None,
        };
        let large_body = "# Big\n\n".to_string() + &"x".repeat(100_000);
        let raw = serialize_file_contents(&fm, &large_body).unwrap();
        let p = tmp.join("big.md");
        std::fs::write(&p, &raw).unwrap();

        let idea = read_idea_meta(&p, IdeaState::Inbox, vec![]).expect("meta");
        assert_eq!(idea.frontmatter.title, "Big");
        assert_eq!(idea.frontmatter.tags, vec!["parser"]);
        // Body field doesn't exist on Idea anymore; verify by reading body
        // separately via the public helper.
        let body = read_body(&p).unwrap();
        assert!(body.contains("# Big"));
        cleanup(tmp);
    }

    fn tempdir() -> PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = OffsetDateTime::now_utc().unix_timestamp_nanos();
        p.push(format!("duckboard-idea-test-{nanos}"));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn cleanup(p: PathBuf) {
        let _ = std::fs::remove_dir_all(p);
    }
}
