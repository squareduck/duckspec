//! Change status progress: partition change-introduced `test:code` scenarios
//! by resolving source `@spec` backlinks.
//!
//! This is a progress snapshot for `ds status <change>`, not an integrity
//! gate. Marker path lists and step checkbox state are not consulted.

use std::path::{Path, PathBuf};

use crate::audit::{self, AuditError, ChangeMergeError, ChangeScenario, ScenarioKey};
use crate::config::Config;

/// Progress snapshot for a single change's `test:code` scenarios.
#[derive(Debug, Default)]
pub struct ChangeCoverage {
    /// Change-introduced `test:code` scenarios with at least one resolving
    /// source `@spec`.
    pub linked: Vec<ScenarioKey>,
    /// Change-introduced `test:code` scenarios with no resolving source
    /// `@spec`.
    pub open: Vec<ScenarioKey>,
    /// Spec deltas that failed to merge/re-parse while projecting scenarios.
    /// Callers (status) print these and continue.
    pub merge_errors: Vec<ChangeMergeError>,
}

/// Errors that prevent a coverage snapshot from being produced.
#[derive(Debug, thiserror::Error)]
pub enum ChangeCoverageError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl From<AuditError> for ChangeCoverageError {
    fn from(err: AuditError) -> Self {
        match err {
            AuditError::Io { path, source } => ChangeCoverageError::Io { path, source },
            // Projection/scan only emit Io today; map other variants if they
            // appear so callers still get a typed error.
            other => ChangeCoverageError::Io {
                path: PathBuf::new(),
                source: std::io::Error::other(other.to_string()),
            },
        }
    }
}

/// Project change-introduced `test:code` scenarios and partition by source
/// backlink resolution. Does not validate artifacts, steps, or exit semantics.
///
/// Linkage is solely whether a source `@spec` key matches the scenario.
/// Marker `> - path:line` lists and step checkboxes are ignored.
pub fn for_change(
    duckspec_root: &Path,
    project_root: &Path,
    config: &Config,
    change_name: &str,
) -> Result<ChangeCoverage, ChangeCoverageError> {
    let canonical_root = duckspec_root
        .canonicalize()
        .map_err(|e| ChangeCoverageError::Io {
            path: duckspec_root.to_path_buf(),
            source: e,
        })?;

    let change_dir = duckspec_root.join("changes").join(change_name);
    let mut merge_errors = Vec::new();

    let scenarios: Vec<ChangeScenario> = audit::build_change_scenarios(
        duckspec_root,
        &canonical_root,
        &change_dir,
        change_name,
        &mut merge_errors,
    )?;

    let backlinks = audit::scan_source_files(project_root, duckspec_root, config)?;
    let backlink_keys = audit::backlink_key_set(&backlinks);

    let mut linked = Vec::new();
    let mut open = Vec::new();

    for s in scenarios {
        if !s.test_code {
            continue;
        }
        // Source resolution only — never inspect marker path lists.
        if backlink_keys.contains(&s.key) {
            linked.push(s.key);
        } else {
            open.push(s.key);
        }
    }

    linked.sort_by_key(|k| k.display());
    open.sort_by_key(|k| k.display());

    Ok(ChangeCoverage {
        linked,
        open,
        merge_errors,
    })
}
