//! Hints appended to the title-summarizer prompt when the first user
//! message is a known duckspec slash command.
//!
//! Each `/ds-*` command gets a one-line hint telling the summarizer what
//! kind of work this chat is about. Most are static; `/ds-apply` reads the
//! current change's step list to surface the actual step being implemented
//! ("implementing step 03-add-login-form" → title becomes "Add login form").
//!
//! Adding a new dynamic hint later is mechanical: swap the static string for
//! a function that inspects `ProjectData` and returns a richer line.
//!
//! Returns `None` when the message isn't a supported `/ds-*` command — the
//! caller sends the summarizer only the raw user/assistant text.

use crate::data::{ProjectData, StepCompletion};

/// Build a single-line hint for the title summariser.
///
/// - `user_msg`: the user's first message in the session.
/// - `scope_key`: the session's on-disk scope (change name for change
///   sessions, exploration id otherwise).
/// - `project`: current project data — consulted for dynamic lookups.
pub fn build_hint(user_msg: &str, scope_key: &str, project: &ProjectData) -> Option<String> {
    let command = extract_slash_command(user_msg)?;
    match command {
        "ds-explore" => Some(
            "user is orienting — gathering context before deciding what to do. \
Summarise what they seem to be exploring."
                .into(),
        ),
        "ds-propose" => Some(
            "user is authoring a proposal for the current change. \
Summarise what's being proposed."
                .into(),
        ),
        "ds-design" => Some(
            "user is authoring a design document. Summarise what's being designed.".into(),
        ),
        "ds-spec" => Some(
            "user is authoring capability specs or spec deltas. \
Summarise which capability area is being specified."
                .into(),
        ),
        "ds-step" => Some(
            "user is breaking the current change into sequential implementation steps. \
Summarise the overall plan."
                .into(),
        ),
        "ds-apply" => Some(apply_hint(scope_key, project)),
        "ds-archive" => Some(
            "user is validating and archiving the current change. \
Summarise what's being archived."
                .into(),
        ),
        "ds-verify" => Some(
            "user is running phase-aware verification on the current change. \
Summarise what's being verified."
                .into(),
        ),
        "ds-codex" => Some(
            "user is drafting or updating codex entries. \
Summarise what knowledge is being captured."
                .into(),
        ),
        _ => None,
    }
}

/// Dynamic hint for `/ds-apply`: surface the label of the next unfinished
/// step so the title can describe what's actually being implemented.
fn apply_hint(scope_key: &str, project: &ProjectData) -> String {
    let fallback = "user is implementing the next step of the current change. \
Summarise what's being implemented.";

    let Some(change) = project
        .active_changes
        .iter()
        .find(|c| c.name == scope_key)
    else {
        return fallback.into();
    };

    let Some(step) = change
        .steps
        .iter()
        .find(|s| !matches!(s.completion, StepCompletion::Done))
    else {
        return fallback.into();
    };

    let label = step.label.trim_end_matches(".md");
    format!(
        "user is implementing step `{label}`. Summarise what this step does \
(infer from its name and the assistant's reply)."
    )
}

/// Pull the first whitespace-delimited token and strip its leading `/`.
/// Returns `None` for messages that don't start with a slash command.
fn extract_slash_command(msg: &str) -> Option<&str> {
    let first = msg.trim_start().split_whitespace().next()?;
    first.strip_prefix('/')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{ChangeData, StepInfo};

    fn project_with_steps(change_name: &str, steps: Vec<StepInfo>) -> ProjectData {
        ProjectData {
            active_changes: vec![ChangeData {
                name: change_name.into(),
                prefix: format!("changes/{change_name}"),
                has_proposal: true,
                has_design: false,
                cap_tree: vec![],
                steps,
            }],
            ..Default::default()
        }
    }

    fn step(label: &str, done: bool) -> StepInfo {
        StepInfo {
            id: format!("changes/foo/steps/{label}"),
            label: label.into(),
            completion: if done {
                StepCompletion::Done
            } else {
                StepCompletion::Partial(0, 1)
            },
        }
    }

    #[test]
    fn extract_slash_command_handles_plain_and_prefixed() {
        assert_eq!(extract_slash_command("/ds-apply"), Some("ds-apply"));
        assert_eq!(
            extract_slash_command("  /ds-propose extra text"),
            Some("ds-propose")
        );
        assert_eq!(extract_slash_command("hello"), None);
        assert_eq!(extract_slash_command(""), None);
    }

    #[test]
    fn returns_none_for_unknown_slash_commands() {
        let project = ProjectData::default();
        assert!(build_hint("/ds-unknown", "any", &project).is_none());
        assert!(build_hint("free-form text", "any", &project).is_none());
    }

    #[test]
    fn returns_static_hint_for_non_apply_commands() {
        let project = ProjectData::default();
        let hint = build_hint("/ds-propose", "foo", &project).unwrap();
        assert!(hint.contains("proposal"));
        assert!(!hint.contains("step"));
    }

    #[test]
    fn apply_hint_surfaces_next_unfinished_step_label() {
        let project = project_with_steps(
            "add-login",
            vec![
                step("01-scaffold.md", true),
                step("02-add-login-form.md", false),
                step("03-wire-up.md", false),
            ],
        );
        let hint = build_hint("/ds-apply", "add-login", &project).unwrap();
        assert!(hint.contains("02-add-login-form"));
        assert!(!hint.contains(".md"), "should strip .md suffix: {hint}");
    }

    #[test]
    fn apply_hint_falls_back_when_change_not_found() {
        let project = ProjectData::default();
        let hint = build_hint("/ds-apply", "ghost-change", &project).unwrap();
        assert!(hint.contains("next step"));
    }

    #[test]
    fn apply_hint_falls_back_when_all_steps_done() {
        let project = project_with_steps(
            "finished",
            vec![
                step("01-a.md", true),
                step("02-b.md", true),
            ],
        );
        let hint = build_hint("/ds-apply", "finished", &project).unwrap();
        assert!(hint.contains("next step"));
    }
}
