//! Stable working-directory strings for agent session storage.
//!
//! Grok keys on-disk sessions by the exact `cwd` string passed to
//! `session/new` / `session/load`. Trailing-slash variants
//! (`/path/proj` vs `/path/proj/`) are different keys — resume with the other
//! form yields `FS_NOT_FOUND`. Normalize once at the boundary so create and
//! resume always agree.

use std::path::{Path, PathBuf};

/// Canonical agent cwd: prefer `canonicalize`, else strip trailing separators.
///
/// Empty input is returned unchanged. A bare root (`/` or `C:\`) is preserved.
pub fn normalize_cwd(path: &Path) -> PathBuf {
    if path.as_os_str().is_empty() {
        return PathBuf::new();
    }
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    strip_trailing_separators(path)
}

fn strip_trailing_separators(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    let mut end = s.len();
    while end > 1 {
        let ch = s[..end].chars().next_back().unwrap();
        if ch == '/' || ch == '\\' {
            end -= ch.len_utf8();
        } else {
            break;
        }
    }
    // Keep a single root slash.
    if end == 0 {
        return PathBuf::from("/");
    }
    PathBuf::from(&s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_trailing_slash_when_path_missing() {
        let p = Path::new("/Users/me/proj/");
        assert_eq!(normalize_cwd(p), PathBuf::from("/Users/me/proj"));
    }

    #[test]
    fn strips_multiple_trailing_slashes() {
        let p = Path::new("/Users/me/proj///");
        assert_eq!(normalize_cwd(p), PathBuf::from("/Users/me/proj"));
    }

    #[test]
    fn leaves_non_trailing_paths_alone_when_missing() {
        let p = Path::new("/Users/me/proj");
        assert_eq!(normalize_cwd(p), PathBuf::from("/Users/me/proj"));
    }

    #[test]
    fn preserves_unix_root() {
        assert_eq!(normalize_cwd(Path::new("/")), PathBuf::from("/"));
        assert_eq!(normalize_cwd(Path::new("///")), PathBuf::from("/"));
    }
}
