//! PATH normalization for GUI launches.
//!
//! When the `.app` bundle is launched from Finder or Dock, macOS launchd hands
//! us a minimal PATH (`/usr/bin:/bin:/usr/sbin:/sbin`). Any tool installed via
//! Homebrew, rustup, uv, or to `~/.local/bin` is invisible, so spawning
//! `claude`, `git`, `rg`, etc. fails with `ENOENT`. We prepend a list of
//! well-known install directories to `PATH` at startup so subprocesses can
//! locate the same tools the user sees in their shell.

use std::collections::HashSet;
use std::path::PathBuf;

/// Prepend well-known bin dirs to `PATH`, preserving existing entries and
/// de-duplicating. Must be called before any threads are spawned — the env
/// mutation is unsafe under the Rust 2024 edition rules.
pub fn augment() {
    let existing = std::env::var_os("PATH").unwrap_or_default();

    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut push = |p: PathBuf| {
        if p.as_os_str().is_empty() {
            return;
        }
        if seen.insert(p.clone()) {
            dirs.push(p);
        }
    };

    for p in well_known_dirs() {
        push(p);
    }
    for p in std::env::split_paths(&existing) {
        push(p);
    }

    match std::env::join_paths(&dirs) {
        Ok(joined) => {
            // SAFETY: called from `main` before any threads are spawned.
            unsafe { std::env::set_var("PATH", &joined) };
            tracing::debug!(path = ?joined, "PATH augmented for subprocess spawns");
        }
        Err(e) => tracing::warn!(%e, "failed to rebuild PATH; leaving as-is"),
    }
}

fn well_known_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        out.push(home.join(".local/bin"));
        out.push(home.join(".claude/local"));
        out.push(home.join(".cargo/bin"));
        out.push(home.join("bin"));
    }
    out.push(PathBuf::from("/opt/homebrew/bin"));
    out.push(PathBuf::from("/opt/homebrew/sbin"));
    out.push(PathBuf::from("/usr/local/bin"));
    out
}
