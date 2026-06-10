//! Detection and resolution of file-path references in rendered text.
//!
//! Shared by the text editor and terminal widgets: both scan the hovered
//! line for cmd-clickable targets. URLs are linkify's job; this module
//! covers `path/to/file.rs:42`-style references. Detection is deliberately
//! loose — an unresolved candidate falls back to the fuzzy file finder
//! pre-filled with the path, so a false positive costs one glance, not a
//! wrong file.

use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use regex::Regex;

/// What a cmd-hover landed on. Shared by the editor and terminal widgets
/// so their click and underline logic can treat URL vs path targets
/// uniformly: solid underline for direct opens (URLs, resolved paths),
/// dashed for paths that will open the fuzzy finder instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    Url(String),
    Path {
        /// The path as written in the text (relative or absolute).
        path: String,
        /// 1-based line number from a `:NN` suffix.
        line: Option<usize>,
        /// True when the path resolves to an existing file.
        exists: bool,
    },
}

impl LinkTarget {
    /// True when cmd-click opens the target outright (a URL or a resolved
    /// path) rather than falling back to the fuzzy file finder.
    pub fn opens_directly(&self) -> bool {
        match self {
            LinkTarget::Url(_) => true,
            LinkTarget::Path { exists, .. } => *exists,
        }
    }
}

/// X-offset/width pairs for drawing a link underline of `width` px: one
/// solid run for targets that open directly, short dashes for paths that
/// fall back to the fuzzy finder — the dashes signal "approximate match".
pub fn underline_segments(width: f32, solid: bool) -> Vec<(f32, f32)> {
    if solid {
        return vec![(0.0, width)];
    }
    const DASH: f32 = 4.0;
    const GAP: f32 = 3.0;
    let mut segments = Vec::new();
    let mut x = 0.0;
    while x < width {
        segments.push((x, DASH.min(width - x)));
        x += DASH + GAP;
    }
    segments
}

/// Root of the currently open project, mirrored here so widgets can resolve
/// relative path references at hover time without threading the root through
/// every editor/terminal construction site. Same global-access pattern as
/// `terminal::current_modifiers`. Written by `State::open_project`.
static PROJECT_ROOT: RwLock<Option<PathBuf>> = RwLock::new(None);

pub fn set_project_root(root: Option<PathBuf>) {
    *PROJECT_ROOT.write().unwrap() = root;
}

fn project_root() -> Option<PathBuf> {
    PROJECT_ROOT.read().unwrap().clone()
}

/// A file-path reference detected in a rendered line of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathRef {
    /// Char offset of the reference's first character within the line.
    pub char_start: usize,
    /// Char offset one past the reference's last character (includes any
    /// `:line:col` suffix so the whole reference is underlined/clickable).
    pub char_end: usize,
    /// The path portion, as written.
    pub path: String,
    /// 1-based line number from a `:NN` suffix.
    pub line: Option<usize>,
    /// True when the path resolves to an existing file.
    pub exists: bool,
}

/// Matches path-shaped tokens: multi-segment paths (`crates/foo/bar.rs`,
/// `/abs/path`, `~/x/y`, `./rel/x`), bare filenames with an extension
/// (`main.rs`, `Cargo.toml`), and dotfiles (`.gitignore`), each with an
/// optional `:line` or `:line:col` suffix (rustc/grep style). Extensions
/// must start with a letter and bare stems need ≥2 chars so version
/// numbers ("0.14") and abbreviations ("e.g.") don't light up.
fn path_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            (?P<path>
                (?: ~/ | \.\./ | \./ | / )?
                [\w.\-]+ (?: / [\w.\-]+ )+
              | \.? [\w\-]{2,} \. [A-Za-z] [A-Za-z0-9]{0,7}
              | \. [A-Za-z] [\w\-]{2,}
            )
            (?: : (?P<line>[0-9]+) (?: : [0-9]+ )? )?
            ",
        )
        .expect("path regex compiles")
    })
}

/// Find a path reference on `line` that contains the char offset
/// `char_col`. Mirrors the linkify scan in the widgets' `detect_link_at`;
/// callers try URLs first and fall back to this.
pub fn detect_path_at(line: &str, char_col: usize) -> Option<PathRef> {
    for caps in path_regex().captures_iter(line) {
        let whole = caps.get(0).unwrap();
        let mut path = caps.name("path").unwrap().as_str();
        let line_no = caps
            .name("line")
            .and_then(|l| l.as_str().parse::<usize>().ok());
        // The greedy segment class eats a sentence-final period
        // ("see src/main.rs."); trim it off the path and the span.
        let mut end = whole.end();
        if line_no.is_none() {
            let trimmed = path.trim_end_matches('.');
            end -= path.len() - trimmed.len();
            path = trimmed;
        }
        if path.is_empty() {
            continue;
        }
        let char_start = line[..whole.start()].chars().count();
        let char_end = line[..end].chars().count();
        if char_col < char_start || char_col >= char_end {
            continue;
        }
        return Some(PathRef {
            char_start,
            char_end,
            path: path.to_string(),
            line: line_no,
            exists: resolve(path).is_some(),
        });
    }
    None
}

/// Resolve a written path to an existing file: `~/` expands to the home
/// directory, absolute paths stand alone, relative paths resolve against
/// the open project's root. Returns `None` when the file doesn't exist.
pub fn resolve(path: &str) -> Option<PathBuf> {
    resolve_with(path, project_root().as_deref())
}

fn resolve_with(path: &str, root: Option<&std::path::Path>) -> Option<PathBuf> {
    let expanded = if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()?.join(rest)
    } else {
        PathBuf::from(path)
    };
    let abs = if expanded.is_absolute() {
        expanded
    } else {
        root?.join(expanded)
    };
    abs.is_file().then_some(abs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detect(line: &str, col: usize) -> Option<(String, Option<usize>)> {
        detect_path_at(line, col).map(|r| (r.path, r.line))
    }

    #[test]
    fn detects_relative_multi_segment_path() {
        assert_eq!(
            detect("see crates/duckboard/src/main.rs for details", 10),
            Some(("crates/duckboard/src/main.rs".into(), None))
        );
    }

    #[test]
    fn detects_line_and_col_suffix() {
        let line = " --> crates/duckboard/src/main.rs:42:5";
        let hit = detect_path_at(line, 10).unwrap();
        assert_eq!(hit.path, "crates/duckboard/src/main.rs");
        assert_eq!(hit.line, Some(42));
        // Span covers the whole reference including the suffix.
        assert_eq!(&line[..hit.char_start], " --> ");
        assert_eq!(hit.char_end, line.chars().count());
    }

    #[test]
    fn detects_bare_filename_and_dotfile() {
        assert_eq!(
            detect("open Cargo.toml now", 7),
            Some(("Cargo.toml".into(), None))
        );
        assert_eq!(
            detect("check .gitignore too", 8),
            Some((".gitignore".into(), None))
        );
    }

    #[test]
    fn detects_backticked_path_without_the_backticks() {
        let line = "edit `src/lib.rs:7` please";
        let hit = detect_path_at(line, 8).unwrap();
        assert_eq!(hit.path, "src/lib.rs");
        assert_eq!(hit.line, Some(7));
        assert_eq!(&line[hit.char_start..hit.char_end], "src/lib.rs:7");
    }

    #[test]
    fn trims_sentence_final_period() {
        let hit = detect_path_at("see src/main.rs.", 6).unwrap();
        assert_eq!(hit.path, "src/main.rs");
        assert_eq!(hit.char_end, "see src/main.rs".chars().count());
    }

    #[test]
    fn ignores_version_numbers_and_abbreviations() {
        assert_eq!(detect("iced 0.14 is great", 6), None);
        assert_eq!(detect("paths, e.g. these", 8), None);
    }

    #[test]
    fn ignores_positions_outside_the_match() {
        assert_eq!(detect("see crates/duckboard/src/main.rs", 2), None);
    }

    #[test]
    fn rg_style_line_with_content_stops_at_suffix() {
        let hit = detect_path_at("src/main.rs:42:5:let x = 1;", 3).unwrap();
        assert_eq!(hit.path, "src/main.rs");
        assert_eq!(hit.line, Some(42));
        assert_eq!(hit.char_end, "src/main.rs:42:5".chars().count());
    }

    #[test]
    fn resolves_relative_against_root_when_file_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(
            resolve_with("src/main.rs", Some(root)),
            Some(root.join("src/main.rs"))
        );
        assert_eq!(resolve_with("src/nope.rs", Some(root)), None);
        assert_eq!(resolve_with("src/main.rs", None), None);
    }

    #[test]
    fn resolves_absolute_paths_without_root() {
        let abs = concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs");
        assert_eq!(resolve_with(abs, None), Some(PathBuf::from(abs)));
    }
}
