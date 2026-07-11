//! Pure helpers for multi-option obvious chrome: ordered options, optional
//! cancel, visibility, key resolution, and chip labels.
//!
//! Product path leaves chrome empty (lifecycle chips retired). Shell remains
//! for a later structured-question fill. Independent of oneshot suggestions —
//! pending oneshot is not a visibility gate.

/// Ordered option actions plus optional cancel.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObviousChrome {
    /// Ordered options in send-text form. Empty ⇒ chrome hidden (unless cancel).
    pub options: Vec<String>,
    /// When set, show cancel chip and bind ⌘⌫ to this send text.
    pub cancel: Option<String>,
}

/// Empty-send form: bare `ds-foo` → `/ds-foo`; already-slashed preserved;
/// blank → `None`.
pub fn format_lifecycle_command(cmd: &str) -> Option<String> {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        None
    } else if cmd.starts_with('/') {
        Some(cmd.to_string())
    } else {
        Some(format!("/{cmd}"))
    }
}

/// Soft-hint / legacy single-command empty-send form.
pub fn bubble_send_text(obvious_command: Option<&str>) -> Option<String> {
    obvious_command.and_then(format_lifecycle_command)
}

pub fn chrome_is_empty(chrome: &ObviousChrome) -> bool {
    chrome.options.is_empty() && chrome.cancel.is_none()
}

/// Idle + empty composer + non-empty chrome. Oneshot pending is not a gate.
pub fn chrome_visible(is_streaming: bool, input_empty: bool, chrome: &ObviousChrome) -> bool {
    !is_streaming && input_empty && !chrome_is_empty(chrome)
}

/// ⌘⌫ target (ignores visibility): cancel send text when set.
pub fn resolve_cmd_backspace(chrome: &ObviousChrome) -> Option<String> {
    chrome.cancel.clone()
}

/// ⌘1…⌘9: 1-based index into options; out of range → None.
pub fn resolve_cmd_digit(chrome: &ObviousChrome, digit: u8) -> Option<String> {
    if !(1..=9).contains(&digit) {
        return None;
    }
    chrome.options.get((digit as usize) - 1).cloned()
}

/// ⌘⌫ when chrome is visible; no-op otherwise.
pub fn resolve_cmd_backspace_when_visible(
    is_streaming: bool,
    input_empty: bool,
    chrome: &ObviousChrome,
) -> Option<String> {
    if !chrome_visible(is_streaming, input_empty, chrome) {
        return None;
    }
    resolve_cmd_backspace(chrome)
}

/// ⌘*n* when chrome is visible; no-op otherwise.
pub fn resolve_cmd_digit_when_visible(
    is_streaming: bool,
    input_empty: bool,
    chrome: &ObviousChrome,
    digit: u8,
) -> Option<String> {
    if !chrome_visible(is_streaming, input_empty, chrome) {
        return None;
    }
    resolve_cmd_digit(chrome, digit)
}

/// Chip label: hotkey then action, e.g. `⌘1  /ds-step`.
pub fn option_chip_label(index_1based: usize, action: &str) -> String {
    format!("⌘{index_1based}  {action}")
}

/// Chip label for cancel: `⌘⌫  <cancel text>`.
pub fn cancel_chip_label(cancel: &str) -> String {
    format!("⌘⌫  {cancel}")
}

/// Spacer height above chrome so chips sit at the bottom of the chat viewport
/// when natural content is shorter than the viewport.
pub fn chrome_bottom_pad(viewport_h: f32, content_h: f32, prev_pad: f32) -> f32 {
    let natural = (content_h - prev_pad).max(0.0);
    (viewport_h - natural).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_options_chrome() -> ObviousChrome {
        ObviousChrome {
            options: vec!["/ds-step".into(), "/ds-spec".into(), "/ds-archive".into()],
            cancel: None,
        }
    }

    fn sample_cancel_chrome() -> ObviousChrome {
        ObviousChrome {
            options: vec!["/ds-step".into(), "/ds-spec".into()],
            cancel: Some("cancel".into()),
        }
    }

    // @spec chat/obvious-bubble Empty-send option formatting: Bare skill name formats with leading slash
    #[test]
    fn bare_skill_name_formats_with_leading_slash() {
        // GIVEN a skill name stored without a leading slash
        // WHEN the empty-send text is derived
        // THEN the send text is that name with a single leading `/`
        assert_eq!(
            format_lifecycle_command("ds-explore").as_deref(),
            Some("/ds-explore")
        );
        assert_eq!(
            format_lifecycle_command("ds-archive").as_deref(),
            Some("/ds-archive")
        );
    }

    // @spec chat/obvious-bubble Empty-send option formatting: Already-slashed command is preserved
    #[test]
    fn already_slashed_command_is_preserved() {
        // GIVEN a skill name that already begins with `/`
        // WHEN the empty-send text is derived
        // THEN the send text equals the stored name
        assert_eq!(
            format_lifecycle_command("/ds-spec").as_deref(),
            Some("/ds-spec")
        );
    }

    // @spec chat/obvious-bubble Chrome visibility: Idle empty composer with non-empty options shows chrome
    #[test]
    fn idle_empty_composer_with_non_empty_options_shows_chrome() {
        // GIVEN non-empty options, empty composer, no main turn
        let chrome = sample_options_chrome();
        assert!(chrome_visible(false, true, &chrome));
    }

    // @spec chat/obvious-bubble Chrome visibility: Streaming hides chrome
    #[test]
    fn streaming_hides_chrome() {
        let chrome = sample_options_chrome();
        assert!(!chrome_visible(true, true, &chrome));
    }

    // @spec chat/obvious-bubble Chrome visibility: Non-empty composer hides chrome
    #[test]
    fn non_empty_composer_hides_chrome() {
        let chrome = sample_options_chrome();
        assert!(!chrome_visible(false, false, &chrome));
    }

    // @spec chat/obvious-bubble Chrome visibility: Empty options hide chrome
    #[test]
    fn empty_options_hide_chrome() {
        let chrome = ObviousChrome::default();
        assert!(chrome_is_empty(&chrome));
        assert!(!chrome_visible(false, true, &chrome));
    }

    // @spec chat/obvious-bubble Key resolution: Cmd-digit sends matching option
    #[test]
    fn cmd_digit_sends_matching_option() {
        let chrome = sample_options_chrome();
        let sent = resolve_cmd_digit_when_visible(false, true, &chrome, 2).expect("visible");
        assert_eq!(sent, "/ds-spec");
    }

    // @spec chat/obvious-bubble Key resolution: Cmd-Backspace sends cancel when set
    #[test]
    fn cmd_backspace_sends_cancel_when_set() {
        let chrome = sample_cancel_chrome();
        let sent = resolve_cmd_backspace_when_visible(false, true, &chrome).expect("visible");
        assert_eq!(sent, "cancel");
    }

    // @spec chat/obvious-bubble Key resolution: Resolution is a no-op when chrome not visible
    #[test]
    fn resolution_is_a_no_op_when_chrome_not_visible() {
        let chrome = sample_cancel_chrome();
        assert_eq!(
            resolve_cmd_backspace_when_visible(true, true, &chrome),
            None
        );
        assert_eq!(
            resolve_cmd_digit_when_visible(true, true, &chrome, 1),
            None
        );
        assert_eq!(
            resolve_cmd_backspace_when_visible(false, false, &chrome),
            None
        );
        let empty = ObviousChrome::default();
        assert_eq!(
            resolve_cmd_digit_when_visible(false, true, &empty, 1),
            None
        );
    }

    // @spec chat/obvious-bubble Chip display: Option chip label is hotkey then action
    #[test]
    fn option_chip_label_is_hotkey_then_action() {
        let action = "/ds-step";
        let label = option_chip_label(1, action);
        assert!(label.starts_with("⌘1"), "label={label}");
        assert!(label.contains(action), "label={label}");
        assert_eq!(action, "/ds-step");
        assert_ne!(label, action);
    }

    // @spec chat/obvious-bubble Chip display: Cancel chip label is hotkey then cancel text
    #[test]
    fn cancel_chip_label_is_hotkey_then_cancel_text() {
        let cancel = "cancel";
        let label = cancel_chip_label(cancel);
        assert!(label.starts_with("⌘⌫"), "label={label}");
        assert!(label.contains(cancel), "label={label}");
        assert_eq!(cancel, "cancel");
        assert_ne!(label, cancel);
    }

    // @spec chat/obvious-bubble Chrome bottom pad: Short content yields positive pad
    #[test]
    fn short_content_yields_positive_pad() {
        assert_eq!(chrome_bottom_pad(400.0, 100.0, 0.0), 300.0);
    }

    // @spec chat/obvious-bubble Chrome bottom pad: Content at or above viewport yields zero pad
    #[test]
    fn content_at_or_above_viewport_yields_zero_pad() {
        assert_eq!(chrome_bottom_pad(400.0, 500.0, 0.0), 0.0);
    }

    // @spec chat/obvious-bubble Ephemeral chrome: Visible chrome is not a stored user message
    #[test]
    fn visible_chrome_is_not_a_stored_user_message() {
        use crate::chat_store::{ChatSession, ContentBlock, Role};

        let chrome = sample_options_chrome();
        assert!(chrome_visible(false, true, &chrome));
        let action = chrome.options.first().cloned().expect("option");

        let session = ChatSession::new("change".into());
        let has_chrome_msg = session.messages.iter().any(|m| {
            matches!(m.role, Role::User)
                && m.content.iter().any(|b| match b {
                    ContentBlock::Text(t) => t == &action || t == &option_chip_label(1, &action),
                    _ => false,
                })
        });
        assert!(
            !has_chrome_msg,
            "chrome must not appear in the transcript until activation"
        );
    }
}
