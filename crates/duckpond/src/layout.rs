use std::path::Path;

/// The kind of artifact a file represents, determined by its path
/// relative to the `duckspec/` root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactKind {
    /// `caps/<path>/spec.md`
    CapSpec,
    /// `caps/<path>/doc.md`
    CapDoc,
    /// `changes/<name>/caps/<path>/spec.md` — new or full-replace spec
    ChangeCapSpec,
    /// `changes/<name>/caps/<path>/doc.md` — new or full-replace doc
    ChangeCapDoc,
    /// `changes/<name>/caps/<path>/spec.delta.md`
    SpecDelta,
    /// `changes/<name>/caps/<path>/doc.delta.md`
    DocDelta,
    /// `changes/<name>/proposal.md`
    Proposal,
    /// `changes/<name>/design.md`
    Design,
    /// `changes/<name>/steps/NN-<slug>.md`
    Step,
    /// `codex/<path>.md`
    Codex,
    /// `project.md`
    Project,
}

/// Classify a file by its path relative to the `duckspec/` root.
///
/// Returns `None` if the path does not match any known artifact
/// pattern (e.g. README files, non-`.md` files, archive contents).
pub fn classify(relative_path: &Path) -> Option<ArtifactKind> {
    let components: Vec<&str> = relative_path
        .components()
        .map(|c| c.as_os_str().to_str().unwrap_or(""))
        .collect();

    if components.is_empty() {
        return None;
    }

    // project.md at the root
    if components.len() == 1 && components[0] == "project.md" {
        return Some(ArtifactKind::Project);
    }

    match components[0] {
        "caps" => classify_caps(&components[1..]),
        "codex" => classify_codex(&components[1..]),
        "changes" => classify_change(&components[1..]),
        _ => None,
    }
}

/// Classify a path under `caps/`.
fn classify_caps(rest: &[&str]) -> Option<ArtifactKind> {
    // Need at least one segment (the filename).
    let filename = *rest.last()?;
    match filename {
        "spec.md" if rest.len() >= 2 => Some(ArtifactKind::CapSpec),
        "doc.md" if rest.len() >= 2 => Some(ArtifactKind::CapDoc),
        _ => None,
    }
}

/// Classify a path under `codex/`.
fn classify_codex(rest: &[&str]) -> Option<ArtifactKind> {
    let filename = *rest.last()?;
    if filename.ends_with(".md") {
        Some(ArtifactKind::Codex)
    } else {
        None
    }
}

/// Classify a path under `changes/<name>/`.
fn classify_change(rest: &[&str]) -> Option<ArtifactKind> {
    // rest[0] is the change name, rest[1..] is the content.
    if rest.len() < 2 {
        return None;
    }
    let within_change = &rest[1..];

    // Direct children of the change folder.
    if within_change.len() == 1 {
        return match within_change[0] {
            "proposal.md" => Some(ArtifactKind::Proposal),
            "design.md" => Some(ArtifactKind::Design),
            _ => None,
        };
    }

    match within_change[0] {
        "caps" => classify_change_caps(&within_change[1..]),
        "steps" => classify_step(&within_change[1..]),
        _ => None,
    }
}

/// Classify a path under `changes/<name>/caps/`.
fn classify_change_caps(rest: &[&str]) -> Option<ArtifactKind> {
    let filename = *rest.last()?;
    // Need at least a capability segment + filename.
    if rest.len() < 2 {
        return None;
    }
    match filename {
        "spec.delta.md" => Some(ArtifactKind::SpecDelta),
        "doc.delta.md" => Some(ArtifactKind::DocDelta),
        "spec.md" => Some(ArtifactKind::ChangeCapSpec),
        "doc.md" => Some(ArtifactKind::ChangeCapDoc),
        _ => None,
    }
}

/// Classify a path under `changes/<name>/steps/`.
fn classify_step(rest: &[&str]) -> Option<ArtifactKind> {
    if rest.len() != 1 {
        return None;
    }
    let filename = rest[0];
    if filename.ends_with(".md") {
        Some(ArtifactKind::Step)
    } else {
        None
    }
}

/// Extract the slug portion from a step filename.
///
/// Step filenames follow the pattern `NN-<slug>.md`. Returns the slug
/// portion, or `None` if the filename doesn't match the pattern.
///
/// ```
/// use duckpond::layout::extract_step_slug;
/// assert_eq!(extract_step_slug("01-scaffold.md"), Some("scaffold".into()));
/// assert_eq!(extract_step_slug("02-implement-enrollment.md"), Some("implement-enrollment".into()));
/// assert_eq!(extract_step_slug("proposal.md"), None);
/// ```
pub fn extract_step_slug(filename: &str) -> Option<String> {
    let stem = filename.strip_suffix(".md")?;
    let (nn, slug) = stem.split_once('-')?;
    // NN must be exactly two digits.
    if nn.len() == 2 && nn.chars().all(|c| c.is_ascii_digit()) && !slug.is_empty() {
        Some(slug.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn project_md() {
        assert_eq!(classify(&p("project.md")), Some(ArtifactKind::Project));
    }

    #[test]
    fn cap_spec() {
        assert_eq!(
            classify(&p("caps/auth/spec.md")),
            Some(ArtifactKind::CapSpec)
        );
    }

    #[test]
    fn cap_doc() {
        assert_eq!(
            classify(&p("caps/auth/doc.md")),
            Some(ArtifactKind::CapDoc)
        );
    }

    #[test]
    fn nested_cap_spec() {
        assert_eq!(
            classify(&p("caps/auth/oauth/spec.md")),
            Some(ArtifactKind::CapSpec)
        );
    }

    #[test]
    fn codex_entry() {
        assert_eq!(
            classify(&p("codex/architecture.md")),
            Some(ArtifactKind::Codex)
        );
    }

    #[test]
    fn nested_codex_entry() {
        assert_eq!(
            classify(&p("codex/domain/billing.md")),
            Some(ArtifactKind::Codex)
        );
    }

    #[test]
    fn proposal() {
        assert_eq!(
            classify(&p("changes/add-oauth/proposal.md")),
            Some(ArtifactKind::Proposal)
        );
    }

    #[test]
    fn design() {
        assert_eq!(
            classify(&p("changes/add-oauth/design.md")),
            Some(ArtifactKind::Design)
        );
    }

    #[test]
    fn change_cap_spec() {
        assert_eq!(
            classify(&p("changes/add-oauth/caps/auth/oauth/spec.md")),
            Some(ArtifactKind::ChangeCapSpec)
        );
    }

    #[test]
    fn change_cap_doc() {
        assert_eq!(
            classify(&p("changes/add-oauth/caps/auth/oauth/doc.md")),
            Some(ArtifactKind::ChangeCapDoc)
        );
    }

    #[test]
    fn spec_delta() {
        assert_eq!(
            classify(&p("changes/add-oauth/caps/auth/spec.delta.md")),
            Some(ArtifactKind::SpecDelta)
        );
    }

    #[test]
    fn doc_delta() {
        assert_eq!(
            classify(&p("changes/add-oauth/caps/auth/doc.delta.md")),
            Some(ArtifactKind::DocDelta)
        );
    }

    #[test]
    fn step() {
        assert_eq!(
            classify(&p("changes/add-oauth/steps/01-scaffold.md")),
            Some(ArtifactKind::Step)
        );
    }

    #[test]
    fn unknown_file_returns_none() {
        assert_eq!(classify(&p("README.md")), None);
        assert_eq!(classify(&p("caps/auth/notes.md")), None);
        assert_eq!(classify(&p("changes/add-oauth/random.txt")), None);
    }

    #[test]
    fn archive_returns_none() {
        // Archive files are not classified — they are frozen.
        assert_eq!(
            classify(&p("archive/2026-03-15-01-add-oauth/proposal.md")),
            None
        );
    }

    #[test]
    fn caps_root_files_return_none() {
        // spec.md directly under caps/ with no capability segment.
        assert_eq!(classify(&p("caps/spec.md")), None);
    }

    #[test]
    fn non_md_codex_returns_none() {
        assert_eq!(classify(&p("codex/diagram.png")), None);
    }

    #[test]
    fn nested_step_returns_none() {
        // Steps don't nest into subdirectories.
        assert_eq!(
            classify(&p("changes/add-oauth/steps/sub/01-foo.md")),
            None
        );
    }

    #[test]
    fn extract_slug_basic() {
        assert_eq!(
            extract_step_slug("01-scaffold.md"),
            Some("scaffold".into())
        );
    }

    #[test]
    fn extract_slug_multi_segment() {
        assert_eq!(
            extract_step_slug("02-implement-enrollment.md"),
            Some("implement-enrollment".into())
        );
    }

    #[test]
    fn extract_slug_not_a_step() {
        assert_eq!(extract_step_slug("proposal.md"), None);
    }

    #[test]
    fn extract_slug_no_md_extension() {
        assert_eq!(extract_step_slug("01-scaffold.txt"), None);
    }

    #[test]
    fn extract_slug_single_digit_prefix() {
        // NN must be exactly two digits.
        assert_eq!(extract_step_slug("1-scaffold.md"), None);
    }

    #[test]
    fn extract_slug_three_digit_prefix() {
        assert_eq!(extract_step_slug("001-scaffold.md"), None);
    }
}
