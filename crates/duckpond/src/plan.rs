use std::path::PathBuf;

/// The result of a successful `create` planning operation.
///
/// Contains the filesystem operations needed to realize the creation.
/// All paths are relative to the `duckspec/` root. Renames are ordered
/// so that executing them sequentially avoids collisions (reverse order
/// for step renumbering).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Files or directories to create.
    pub creates: Vec<PathBuf>,
    /// Files to rename: `(from, to)`. Ordered for safe sequential execution.
    pub renames: Vec<(PathBuf, PathBuf)>,
}

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("change '{name}' already exists")]
    ChangeExists { name: String },

    #[error("archived change '{name}' already exists (in '{archive_entry}')")]
    ChangeArchived {
        name: String,
        archive_entry: String,
    },

    #[error("change '{name}' not found")]
    ChangeNotFound { name: String },

    #[error("'{path}' already exists in change")]
    ArtifactExists { path: PathBuf },

    #[error("invalid capability path: empty segment")]
    EmptyCapSegment,

    #[error("step slug '{slug}' already exists in change")]
    StepSlugExists { slug: String },

    #[error("step '--after {slug}' not found in change")]
    AfterStepNotFound { slug: String },

    #[error("unknown stage '{stage}'")]
    UnknownStage { stage: String },

    #[error("hook already exists: {path}")]
    HookExists { path: PathBuf },
}

/// Known stage names for hooks.
pub const STAGES: &[&str] = &[
    "explore", "propose", "design", "spec", "step", "apply", "archive", "verify", "codex",
];

/// Position of a hook relative to the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookPosition {
    Pre,
    Post,
}

impl HookPosition {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookPosition::Pre => "pre",
            HookPosition::Post => "post",
        }
    }
}

/// Plan the creation of a new change directory.
///
/// `active_changes` — names of directories under `changes/`.
/// `archive_entries` — names of directories under `archive/` (with date prefix).
pub fn create_change(
    name: &str,
    active_changes: &[String],
    archive_entries: &[String],
) -> Result<Plan, PlanError> {
    if active_changes.iter().any(|c| c == name) {
        return Err(PlanError::ChangeExists {
            name: name.to_string(),
        });
    }

    // Archive entries have the form YYYY-MM-DD-NN-<name>.
    // Strip the date+counter prefix to compare.
    for entry in archive_entries {
        if let Some(archived_name) = strip_archive_prefix(entry)
            && archived_name == name {
                return Err(PlanError::ChangeArchived {
                    name: name.to_string(),
                    archive_entry: entry.clone(),
                });
            }
    }

    Ok(Plan {
        creates: vec![PathBuf::from(format!("changes/{name}"))],
        renames: vec![],
    })
}

/// Plan the creation of `proposal.md` in a change.
///
/// `active_changes` — names of directories under `changes/`.
/// `change_files` — filenames directly in `changes/<change>/`.
pub fn create_proposal(
    change: &str,
    active_changes: &[String],
    change_files: &[String],
) -> Result<Plan, PlanError> {
    check_change_exists(change, active_changes)?;

    let artifact = "proposal.md";
    if change_files.iter().any(|f| f == artifact) {
        return Err(PlanError::ArtifactExists {
            path: PathBuf::from(format!("changes/{change}/{artifact}")),
        });
    }

    Ok(Plan {
        creates: vec![PathBuf::from(format!("changes/{change}/{artifact}"))],
        renames: vec![],
    })
}

/// Plan the creation of `design.md` in a change.
///
/// `active_changes` — names of directories under `changes/`.
/// `change_files` — filenames directly in `changes/<change>/`.
pub fn create_design(
    change: &str,
    active_changes: &[String],
    change_files: &[String],
) -> Result<Plan, PlanError> {
    check_change_exists(change, active_changes)?;

    let artifact = "design.md";
    if change_files.iter().any(|f| f == artifact) {
        return Err(PlanError::ArtifactExists {
            path: PathBuf::from(format!("changes/{change}/{artifact}")),
        });
    }

    Ok(Plan {
        creates: vec![PathBuf::from(format!("changes/{change}/{artifact}"))],
        renames: vec![],
    })
}

/// Plan the creation of a spec file (full or delta) for a capability in a change.
///
/// `active_changes` — names of directories under `changes/`.
/// `toplevel_caps` — capability paths that exist under top-level `caps/`
///   (e.g. `["auth", "auth/google", "payments/stripe"]`).
/// `change_cap_files` — files that already exist under `changes/<change>/caps/<cap_path>/`
///   (e.g. `["spec.md", "doc.delta.md"]`).
pub fn create_spec(
    cap_path: &str,
    change: &str,
    active_changes: &[String],
    toplevel_caps: &[String],
    change_cap_files: &[String],
) -> Result<Plan, PlanError> {
    check_change_exists(change, active_changes)?;
    validate_cap_path(cap_path)?;

    let is_existing_cap = toplevel_caps.iter().any(|c| c == cap_path);
    let filename = if is_existing_cap {
        "spec.delta.md"
    } else {
        "spec.md"
    };

    if change_cap_files.iter().any(|f| f == filename) {
        return Err(PlanError::ArtifactExists {
            path: PathBuf::from(format!("changes/{change}/caps/{cap_path}/{filename}")),
        });
    }

    Ok(Plan {
        creates: vec![PathBuf::from(format!(
            "changes/{change}/caps/{cap_path}/{filename}"
        ))],
        renames: vec![],
    })
}

/// Plan the creation of a doc file (full or delta) for a capability in a change.
///
/// Arguments mirror [`create_spec`].
pub fn create_doc(
    cap_path: &str,
    change: &str,
    active_changes: &[String],
    toplevel_caps: &[String],
    change_cap_files: &[String],
) -> Result<Plan, PlanError> {
    check_change_exists(change, active_changes)?;
    validate_cap_path(cap_path)?;

    let is_existing_cap = toplevel_caps.iter().any(|c| c == cap_path);
    let filename = if is_existing_cap {
        "doc.delta.md"
    } else {
        "doc.md"
    };

    if change_cap_files.iter().any(|f| f == filename) {
        return Err(PlanError::ArtifactExists {
            path: PathBuf::from(format!("changes/{change}/caps/{cap_path}/{filename}")),
        });
    }

    Ok(Plan {
        creates: vec![PathBuf::from(format!(
            "changes/{change}/caps/{cap_path}/{filename}"
        ))],
        renames: vec![],
    })
}

/// Plan the creation of a step file in a change, with optional insertion
/// after an existing step.
///
/// `active_changes` — names of directories under `changes/`.
/// `existing_steps` — filenames in `changes/<change>/steps/`, sorted.
///   Each follows the `NN-<slug>.md` pattern.
/// `name` — human name for the step (will be slugified).
/// `after` — if `Some`, the slug of the step to insert after.
pub fn create_step(
    name: &str,
    change: &str,
    active_changes: &[String],
    existing_steps: &[String],
    after: Option<&str>,
) -> Result<Plan, PlanError> {
    check_change_exists(change, active_changes)?;

    let slug = slugify(name);
    let parsed = parse_steps(existing_steps);

    // Check slug uniqueness.
    if parsed.iter().any(|s| s.slug == slug) {
        return Err(PlanError::StepSlugExists { slug });
    }

    let mut creates = Vec::new();
    let mut renames = Vec::new();

    match after {
        None => {
            // Append: next number after the highest existing.
            let next_nn = parsed.last().map_or(1, |s| s.nn + 1);
            creates.push(step_path(change, next_nn, &slug));
        }
        Some(after_slug) => {
            // Find the step to insert after.
            let after_idx = parsed
                .iter()
                .position(|s| s.slug == after_slug)
                .ok_or_else(|| PlanError::AfterStepNotFound {
                    slug: after_slug.to_string(),
                })?;

            let insert_nn = parsed[after_idx].nn + 1;

            // Renumber all steps after the insertion point (reverse order
            // to avoid collisions).
            for s in parsed[after_idx + 1..].iter().rev() {
                let old = step_path(change, s.nn, &s.slug);
                let new = step_path(change, s.nn + 1, &s.slug);
                renames.push((old, new));
            }

            creates.push(step_path(change, insert_nn, &slug));
        }
    }

    Ok(Plan { creates, renames })
}

/// Plan the creation of a hook file.
///
/// `stage` — the stage name (e.g. "explore", "spec").
/// `position` — pre or post.
/// `existing_hooks` — filenames in `hooks/`.
pub fn create_hook(
    stage: &str,
    position: HookPosition,
    existing_hooks: &[String],
) -> Result<Plan, PlanError> {
    if !STAGES.contains(&stage) {
        return Err(PlanError::UnknownStage {
            stage: stage.to_string(),
        });
    }

    let filename = format!("{stage}-{}.md", position.as_str());
    let path = PathBuf::from(format!("hooks/{filename}"));

    if existing_hooks.iter().any(|f| f == &filename) {
        return Err(PlanError::HookExists { path });
    }

    Ok(Plan {
        creates: vec![path],
        renames: vec![],
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn check_change_exists(change: &str, active_changes: &[String]) -> Result<(), PlanError> {
    if !active_changes.iter().any(|c| c == change) {
        return Err(PlanError::ChangeNotFound {
            name: change.to_string(),
        });
    }
    Ok(())
}

fn validate_cap_path(cap_path: &str) -> Result<(), PlanError> {
    for segment in cap_path.split('/') {
        if segment.is_empty() {
            return Err(PlanError::EmptyCapSegment);
        }
    }
    Ok(())
}

/// Strip the `YYYY-MM-DD-NN-` prefix from an archive entry name.
fn strip_archive_prefix(entry: &str) -> Option<&str> {
    // Format: YYYY-MM-DD-NN-<name>
    // Prefix is 14 chars: "2026-03-15-01-", name starts at index 14.
    if entry.len() > 14 && entry.as_bytes()[13] == b'-' {
        let prefix = &entry[..13];
        let parts: Vec<&str> = prefix.split('-').collect();
        if parts.len() == 4
            && parts[0].len() == 4
            && parts[1].len() == 2
            && parts[2].len() == 2
            && parts[3].len() == 2
            && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
        {
            return Some(&entry[14..]);
        }
    }
    None
}

/// Convert a step name to a kebab-case slug.
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

struct ParsedStep {
    nn: u32,
    slug: String,
}

fn parse_steps(filenames: &[String]) -> Vec<ParsedStep> {
    let mut steps: Vec<ParsedStep> = filenames
        .iter()
        .filter_map(|f| {
            let stem = f.strip_suffix(".md")?;
            let (nn_str, slug) = stem.split_once('-')?;
            if nn_str.len() == 2 && nn_str.chars().all(|c| c.is_ascii_digit()) {
                Some(ParsedStep {
                    nn: nn_str.parse().ok()?,
                    slug: slug.to_string(),
                })
            } else {
                None
            }
        })
        .collect();
    steps.sort_by_key(|s| s.nn);
    steps
}

fn step_path(change: &str, nn: u32, slug: &str) -> PathBuf {
    PathBuf::from(format!("changes/{change}/steps/{nn:02}-{slug}.md"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> String {
        v.to_string()
    }

    fn ss(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    // -- create_change --------------------------------------------------------

    #[test]
    fn change_ok() {
        let plan = create_change("add-oauth", &[], &[]).unwrap();
        assert_eq!(plan.creates, vec![PathBuf::from("changes/add-oauth")]);
        assert!(plan.renames.is_empty());
    }

    #[test]
    fn change_exists() {
        let err = create_change("add-oauth", &[s("add-oauth")], &[]).unwrap_err();
        assert!(matches!(err, PlanError::ChangeExists { .. }));
    }

    #[test]
    fn change_archived() {
        let err = create_change(
            "add-oauth",
            &[],
            &[s("2026-03-15-01-add-oauth")],
        )
        .unwrap_err();
        assert!(matches!(err, PlanError::ChangeArchived { .. }));
    }

    // -- create_proposal ------------------------------------------------------

    #[test]
    fn proposal_ok() {
        let plan = create_proposal("add-oauth", &[s("add-oauth")], &[]).unwrap();
        assert_eq!(
            plan.creates,
            vec![PathBuf::from("changes/add-oauth/proposal.md")]
        );
    }

    #[test]
    fn proposal_change_missing() {
        let err = create_proposal("nope", &[], &[]).unwrap_err();
        assert!(matches!(err, PlanError::ChangeNotFound { .. }));
    }

    #[test]
    fn proposal_already_exists() {
        let err = create_proposal(
            "add-oauth",
            &[s("add-oauth")],
            &[s("proposal.md")],
        )
        .unwrap_err();
        assert!(matches!(err, PlanError::ArtifactExists { .. }));
    }

    // -- create_design --------------------------------------------------------

    #[test]
    fn design_ok() {
        let plan = create_design("add-oauth", &[s("add-oauth")], &[]).unwrap();
        assert_eq!(
            plan.creates,
            vec![PathBuf::from("changes/add-oauth/design.md")]
        );
    }

    #[test]
    fn design_already_exists() {
        let err = create_design(
            "add-oauth",
            &[s("add-oauth")],
            &[s("design.md")],
        )
        .unwrap_err();
        assert!(matches!(err, PlanError::ArtifactExists { .. }));
    }

    // -- create_spec ----------------------------------------------------------

    #[test]
    fn spec_new_cap() {
        let plan = create_spec(
            "auth/google",
            "add-oauth",
            &[s("add-oauth")],
            &[s("auth")],
            &[],
        )
        .unwrap();
        assert_eq!(
            plan.creates,
            vec![PathBuf::from("changes/add-oauth/caps/auth/google/spec.md")]
        );
    }

    #[test]
    fn spec_existing_cap_creates_delta() {
        let plan = create_spec(
            "auth",
            "add-oauth",
            &[s("add-oauth")],
            &[s("auth")],
            &[],
        )
        .unwrap();
        assert_eq!(
            plan.creates,
            vec![PathBuf::from(
                "changes/add-oauth/caps/auth/spec.delta.md"
            )]
        );
    }

    #[test]
    fn spec_already_exists_in_change() {
        let err = create_spec(
            "auth",
            "add-oauth",
            &[s("add-oauth")],
            &[s("auth")],
            &[s("spec.delta.md")],
        )
        .unwrap_err();
        assert!(matches!(err, PlanError::ArtifactExists { .. }));
    }

    #[test]
    fn spec_empty_cap_segment() {
        let err = create_spec(
            "auth//google",
            "add-oauth",
            &[s("add-oauth")],
            &[],
            &[],
        )
        .unwrap_err();
        assert!(matches!(err, PlanError::EmptyCapSegment));
    }

    // -- create_doc -----------------------------------------------------------

    #[test]
    fn doc_new_cap() {
        let plan = create_doc(
            "auth/google",
            "add-oauth",
            &[s("add-oauth")],
            &[s("auth")],
            &[],
        )
        .unwrap();
        assert_eq!(
            plan.creates,
            vec![PathBuf::from("changes/add-oauth/caps/auth/google/doc.md")]
        );
    }

    #[test]
    fn doc_existing_cap_creates_delta() {
        let plan = create_doc(
            "auth",
            "add-oauth",
            &[s("add-oauth")],
            &[s("auth")],
            &[],
        )
        .unwrap();
        assert_eq!(
            plan.creates,
            vec![PathBuf::from(
                "changes/add-oauth/caps/auth/doc.delta.md"
            )]
        );
    }

    // -- create_step ----------------------------------------------------------

    #[test]
    fn step_append_to_empty() {
        let plan = create_step(
            "scaffold",
            "add-oauth",
            &[s("add-oauth")],
            &[],
            None,
        )
        .unwrap();
        assert_eq!(
            plan.creates,
            vec![PathBuf::from(
                "changes/add-oauth/steps/01-scaffold.md"
            )]
        );
        assert!(plan.renames.is_empty());
    }

    #[test]
    fn step_append_to_existing() {
        let plan = create_step(
            "implement auth",
            "add-oauth",
            &[s("add-oauth")],
            &ss(&["01-scaffold.md", "02-database.md"]),
            None,
        )
        .unwrap();
        assert_eq!(
            plan.creates,
            vec![PathBuf::from(
                "changes/add-oauth/steps/03-implement-auth.md"
            )]
        );
        assert!(plan.renames.is_empty());
    }

    #[test]
    fn step_insert_after_with_renumbering() {
        let plan = create_step(
            "middleware",
            "add-oauth",
            &[s("add-oauth")],
            &ss(&["01-scaffold.md", "02-database.md", "03-ui.md"]),
            Some("scaffold"),
        )
        .unwrap();

        // New step at 02.
        assert_eq!(
            plan.creates,
            vec![PathBuf::from(
                "changes/add-oauth/steps/02-middleware.md"
            )]
        );

        // Renames in reverse order: 03 -> 04 first, then 02 -> 03.
        assert_eq!(
            plan.renames,
            vec![
                (
                    PathBuf::from("changes/add-oauth/steps/03-ui.md"),
                    PathBuf::from("changes/add-oauth/steps/04-ui.md"),
                ),
                (
                    PathBuf::from("changes/add-oauth/steps/02-database.md"),
                    PathBuf::from("changes/add-oauth/steps/03-database.md"),
                ),
            ]
        );
    }

    #[test]
    fn step_insert_after_last_no_renumber() {
        let plan = create_step(
            "deploy",
            "add-oauth",
            &[s("add-oauth")],
            &ss(&["01-scaffold.md", "02-database.md"]),
            Some("database"),
        )
        .unwrap();
        assert_eq!(
            plan.creates,
            vec![PathBuf::from(
                "changes/add-oauth/steps/03-deploy.md"
            )]
        );
        assert!(plan.renames.is_empty());
    }

    #[test]
    fn step_slug_conflict() {
        let err = create_step(
            "scaffold",
            "add-oauth",
            &[s("add-oauth")],
            &ss(&["01-scaffold.md"]),
            None,
        )
        .unwrap_err();
        assert!(matches!(err, PlanError::StepSlugExists { .. }));
    }

    #[test]
    fn step_after_not_found() {
        let err = create_step(
            "deploy",
            "add-oauth",
            &[s("add-oauth")],
            &ss(&["01-scaffold.md"]),
            Some("nope"),
        )
        .unwrap_err();
        assert!(matches!(err, PlanError::AfterStepNotFound { .. }));
    }

    // -- strip_archive_prefix -------------------------------------------------

    #[test]
    fn archive_prefix_valid() {
        assert_eq!(
            strip_archive_prefix("2026-03-15-01-add-oauth"),
            Some("add-oauth")
        );
    }

    #[test]
    fn archive_prefix_invalid() {
        assert_eq!(strip_archive_prefix("not-an-archive"), None);
    }

    // -- create_hook ----------------------------------------------------------

    #[test]
    fn hook_pre_ok() {
        let plan = create_hook("explore", HookPosition::Pre, &[]).unwrap();
        assert_eq!(plan.creates, vec![PathBuf::from("hooks/explore-pre.md")]);
    }

    #[test]
    fn hook_post_ok() {
        let plan = create_hook("spec", HookPosition::Post, &[]).unwrap();
        assert_eq!(plan.creates, vec![PathBuf::from("hooks/spec-post.md")]);
    }

    #[test]
    fn hook_unknown_stage() {
        let err = create_hook("bogus", HookPosition::Pre, &[]).unwrap_err();
        assert!(matches!(err, PlanError::UnknownStage { .. }));
    }

    #[test]
    fn hook_already_exists() {
        let err = create_hook(
            "explore",
            HookPosition::Pre,
            &[s("explore-pre.md")],
        )
        .unwrap_err();
        assert!(matches!(err, PlanError::HookExists { .. }));
    }

    // -- slugify --------------------------------------------------------------

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Implement Auth"), "implement-auth");
    }

    #[test]
    fn slugify_extra_spaces() {
        assert_eq!(slugify("  set  up   database  "), "set-up-database");
    }
}
