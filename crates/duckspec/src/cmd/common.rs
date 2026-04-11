use std::path::{Path, PathBuf};

/// Resolve a user-provided path string to an existing filesystem path.
///
/// Tries in order:
/// 1. As an absolute path (if it starts with `/`).
/// 2. Relative to the current working directory.
/// 3. Relative to the `duckspec/` root.
///
/// Returns the first path that exists on the filesystem. Errors if none
/// of the candidates exist.
pub fn resolve_path(input: &str, duckspec_root: &Path) -> anyhow::Result<PathBuf> {
    let p = PathBuf::from(input);

    // 1. Absolute path.
    if p.is_absolute() {
        if p.exists() {
            return Ok(p);
        }
        anyhow::bail!("path does not exist: {}", p.display());
    }

    // 2. Relative to CWD.
    let cwd = std::env::current_dir()?;
    let from_cwd = cwd.join(&p);
    if from_cwd.exists() {
        return Ok(from_cwd);
    }

    // 3. Relative to duckspec root.
    let from_root = duckspec_root.join(&p);
    if from_root.exists() {
        return Ok(from_root);
    }

    anyhow::bail!(
        "path not found: {input}\n  tried: {}\n  tried: {}",
        from_cwd.display(),
        from_root.display()
    );
}

/// Walk up from the current directory looking for a `duckspec/` folder.
pub fn find_duckspec_root() -> anyhow::Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        let candidate = dir.join("duckspec");
        if candidate.is_dir() {
            return Ok(candidate);
        }
        if !dir.pop() {
            anyhow::bail!("no duckspec/ directory found in any parent directory");
        }
    }
}

/// Collect `.md` files from a path. If it's a file, returns just that file.
/// If it's a directory, walks it recursively.
pub fn collect_files(scan_path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if scan_path.is_file() {
        return Ok(vec![scan_path.to_path_buf()]);
    }

    let mut files = Vec::new();
    walk_dir(scan_path, &mut files)?;
    files.sort();
    Ok(files)
}

/// Recursively collect `.md` files from a directory.
pub fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| anyhow::anyhow!("failed to read directory {}: {e}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
    Ok(())
}

/// Collect subdirectory names directly under a directory.
/// Returns sorted names. Skips non-directory entries.
pub fn list_subdirs(dir: &Path) -> anyhow::Result<Vec<String>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(dir)
        .map_err(|e| anyhow::anyhow!("failed to read directory {}: {e}", dir.display()))?;

    let mut names = Vec::new();
    for entry in entries {
        let entry = entry?;
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}
