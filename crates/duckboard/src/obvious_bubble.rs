//! Pure helpers for the lifecycle obvious bubble: send text and visibility.
//!
//! The bubble path reads only `obvious_command` (and idle/empty gates). It is
//! independent of oneshot composer suggestions — pending oneshot is not a
//! visibility gate.

/// Empty-send form of the lifecycle command: bare names get a leading `/`;
/// already-slashed values are preserved; absent or blank yields `None`.
pub fn bubble_send_text(obvious_command: Option<&str>) -> Option<String> {
    match obvious_command.map(str::trim).filter(|s| !s.is_empty()) {
        Some(cmd) if cmd.starts_with('/') => Some(cmd.to_string()),
        Some(cmd) => Some(format!("/{cmd}")),
        None => None,
    }
}

/// Whether the obvious bubble should be shown.
///
/// Gates: not streaming, empty composer, non-empty lifecycle command.
/// Oneshot pending is intentionally not a parameter — it does not hide the
/// bubble when the other gates hold.
pub fn bubble_visible(
    is_streaming: bool,
    input_empty: bool,
    obvious_command: Option<&str>,
) -> bool {
    !is_streaming
        && input_empty
        && obvious_command.map(str::trim).is_some_and(|s| !s.is_empty())
}

/// Text to send on bubble activation (⌘↩ / click), or `None` when the bubble
/// is not visible (activation is a no-op).
///
/// Uses only lifecycle `obvious_command` — never oneshot default-prompt list
/// entries, even when that list is non-empty and differs.
pub fn activation_send_text(
    is_streaming: bool,
    input_empty: bool,
    obvious_command: Option<&str>,
) -> Option<String> {
    if !bubble_visible(is_streaming, input_empty, obvious_command) {
        return None;
    }
    bubble_send_text(obvious_command)
}

#[cfg(test)]
mod tests {
    use super::*;

    // @spec chat/obvious-bubble Lifecycle send text: Bare skill name formats with leading slash
    #[test]
    fn bare_skill_name_formats_with_leading_slash() {
        // GIVEN a lifecycle next command stored without a leading slash
        // WHEN the bubble send text is derived
        // THEN the send text is that command with a single leading `/`
        assert_eq!(
            bubble_send_text(Some("ds-explore")).as_deref(),
            Some("/ds-explore")
        );
        assert_eq!(
            bubble_send_text(Some("ds-archive")).as_deref(),
            Some("/ds-archive")
        );
    }

    // @spec chat/obvious-bubble Lifecycle send text: Already-slashed command is preserved
    #[test]
    fn already_slashed_command_is_preserved() {
        // GIVEN a lifecycle next command that already begins with `/`
        // WHEN the bubble send text is derived
        // THEN the send text equals the stored command
        assert_eq!(
            bubble_send_text(Some("/ds-spec")).as_deref(),
            Some("/ds-spec")
        );
    }

    // @spec chat/obvious-bubble Lifecycle send text: Absent command yields no send text
    #[test]
    fn absent_command_yields_no_send_text() {
        // GIVEN no lifecycle next command for the session
        // WHEN the bubble send text is derived
        // THEN there is no send text
        assert_eq!(bubble_send_text(None), None);
        assert_eq!(bubble_send_text(Some("")), None);
        assert_eq!(bubble_send_text(Some("   ")), None);
    }

    // @spec chat/obvious-bubble Bubble visibility: Idle empty composer with command shows bubble
    #[test]
    fn idle_empty_composer_with_command_shows_bubble() {
        // GIVEN a lifecycle next command, empty composer, no main turn in progress
        // WHEN bubble visibility is evaluated
        // THEN the bubble is shown
        assert!(bubble_visible(false, true, Some("ds-apply")));
    }

    // @spec chat/obvious-bubble Bubble visibility: Streaming hides bubble
    #[test]
    fn streaming_hides_bubble() {
        // GIVEN a lifecycle next command, empty composer, main turn in progress
        // WHEN bubble visibility is evaluated
        // THEN the bubble is not shown
        assert!(!bubble_visible(true, true, Some("ds-apply")));
    }

    // @spec chat/obvious-bubble Bubble visibility: Non-empty composer hides bubble
    #[test]
    fn non_empty_composer_hides_bubble() {
        // GIVEN a lifecycle next command, non-empty composer, no main turn
        // WHEN bubble visibility is evaluated
        // THEN the bubble is not shown
        assert!(!bubble_visible(false, false, Some("ds-apply")));
    }

    // @spec chat/obvious-bubble Bubble visibility: No command hides bubble
    #[test]
    fn no_command_hides_bubble() {
        // GIVEN no lifecycle next command, empty composer, no main turn
        // WHEN bubble visibility is evaluated
        // THEN the bubble is not shown
        assert!(!bubble_visible(false, true, None));
        assert!(!bubble_visible(false, true, Some("")));
        assert!(!bubble_visible(false, true, Some("  ")));
    }

    // @spec chat/obvious-bubble Bubble visibility: Oneshot pending does not hide bubble when otherwise visible
    #[test]
    fn oneshot_pending_does_not_hide_bubble_when_otherwise_visible() {
        // GIVEN a lifecycle next command, empty composer, no main turn, and a
        // pending reply-suggestion oneshot — oneshot state is not a visibility
        // gate, so the pure API has no pending parameter.
        // WHEN bubble visibility is evaluated under the idle/empty/command gates
        // THEN the bubble is shown (pending would not hide it)
        assert!(bubble_visible(false, true, Some("ds-apply")));
    }

    // @spec chat/obvious-bubble Activation send: Activation sends lifecycle text when visible
    #[test]
    fn activation_sends_lifecycle_text_when_visible() {
        // GIVEN the obvious bubble is visible with a derived bubble send text
        // WHEN the bubble is activated
        // THEN a user message is sent whose text is the bubble send text
        let text = activation_send_text(false, true, Some("ds-archive"));
        assert_eq!(text.as_deref(), Some("/ds-archive"));
    }

    // @spec chat/obvious-bubble Activation send: Activation is a no-op when not visible
    #[test]
    fn activation_is_a_no_op_when_not_visible() {
        // GIVEN the obvious bubble is not visible (streaming / typing / no cmd)
        // WHEN bubble activation is requested
        // THEN no message is sent
        assert_eq!(activation_send_text(true, true, Some("ds-apply")), None);
        assert_eq!(activation_send_text(false, false, Some("ds-apply")), None);
        assert_eq!(activation_send_text(false, true, None), None);
    }

    // @spec chat/obvious-bubble Activation send: Send text ignores oneshot list when both differ
    #[test]
    fn send_text_ignores_oneshot_list_when_both_differ() {
        // GIVEN visible bubble with lifecycle send text A, and a non-empty
        // oneshot list whose active entry is B (A ≠ B). Activation does not
        // take oneshot entries as input — only obvious_command.
        // WHEN the bubble is activated
        // THEN the sent text is A, not B
        let lifecycle = Some("ds-archive");
        let oneshot_active = "/ds-review";
        let sent = activation_send_text(false, true, lifecycle).expect("visible");
        assert_eq!(sent, "/ds-archive");
        assert_ne!(sent, oneshot_active);
    }

    // @spec chat/obvious-bubble Ephemeral chrome: Visible bubble is not a stored user message
    #[test]
    fn visible_bubble_is_not_a_stored_user_message() {
        // GIVEN the obvious bubble is shown and has not been activated
        // WHEN the session transcript is inspected
        // THEN it does not contain a user message for the ghost bubble.
        // Chrome helpers only derive visibility/send text — they never append
        // ChatMessage rows. A fresh session stays free of ghost user messages.
        use crate::chat_store::{ChatSession, ContentBlock, Role};

        let cmd = Some("ds-apply");
        assert!(bubble_visible(false, true, cmd));
        let ghost_text = bubble_send_text(cmd).expect("send form");

        let session = ChatSession::new("change".into());
        let has_ghost = session.messages.iter().any(|m| {
            matches!(m.role, Role::User)
                && m.content.iter().any(|b| match b {
                    ContentBlock::Text(t) => t == &ghost_text,
                    _ => false,
                })
        });
        assert!(!has_ghost, "ghost chrome must not appear in the transcript");
    }
}
