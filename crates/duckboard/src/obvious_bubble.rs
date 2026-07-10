//! Pure helpers for multi-option obvious chrome: lifecycle `/ds-*`, affirm,
//! decline — visibility, key resolution, and chip labels.
//!
//! Independent of oneshot composer suggestions — pending oneshot is not a
//! visibility gate.

/// Ordered lifecycle actions plus optional affirm/decline gate.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObviousChrome {
    /// Ordered lifecycle actions in empty-send form (`/ds-step`, …).
    pub lifecycle: Vec<String>,
    /// Affirm row: Confirm (pre-step gate), Commit (post-archive dirty), or
    /// Create change (nonempty exploration).
    pub affirm: Option<Affirm>,
    /// When true, show Reject and bind ⌘⌫.
    pub decline: bool,
}

/// Affirm action shown on ⌘↩ when Confirm, Commit, or Create change is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Affirm {
    Confirm,
    Commit,
    /// Nonempty exploration handoff — send text is literal `Create change`.
    CreateChange,
}

impl Affirm {
    pub fn send_text(self) -> &'static str {
        match self {
            Affirm::Confirm => "Confirm",
            Affirm::Commit => "Commit",
            Affirm::CreateChange => "Create change",
        }
    }
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
///
/// Prefer [`format_lifecycle_command`] for bare names; this accepts `Option`
/// for oneshot heuristic helpers.
pub fn bubble_send_text(obvious_command: Option<&str>) -> Option<String> {
    obvious_command.and_then(format_lifecycle_command)
}

pub fn chrome_is_empty(chrome: &ObviousChrome) -> bool {
    chrome.lifecycle.is_empty() && chrome.affirm.is_none() && !chrome.decline
}

/// Idle + empty composer + non-empty chrome. Oneshot pending is not a gate.
pub fn chrome_visible(
    is_streaming: bool,
    input_empty: bool,
    chrome: &ObviousChrome,
) -> bool {
    !is_streaming && input_empty && !chrome_is_empty(chrome)
}

/// ⌘↩ target (ignores visibility): affirm if present, else lifecycle[0].
pub fn resolve_cmd_enter(chrome: &ObviousChrome) -> Option<String> {
    if let Some(affirm) = chrome.affirm {
        return Some(affirm.send_text().to_string());
    }
    chrome.lifecycle.first().cloned()
}

/// ⌘⌫ target (ignores visibility): `Reject` when decline is set.
pub fn resolve_cmd_backspace(chrome: &ObviousChrome) -> Option<String> {
    if chrome.decline {
        Some("Reject".to_string())
    } else {
        None
    }
}

/// ⌘1…⌘9: 1-based index into lifecycle; out of range → None.
pub fn resolve_cmd_digit(chrome: &ObviousChrome, digit: u8) -> Option<String> {
    if !(1..=9).contains(&digit) {
        return None;
    }
    chrome.lifecycle.get((digit as usize) - 1).cloned()
}

/// ⌘↩ when chrome is visible; no-op otherwise.
pub fn resolve_cmd_enter_when_visible(
    is_streaming: bool,
    input_empty: bool,
    chrome: &ObviousChrome,
) -> Option<String> {
    if !chrome_visible(is_streaming, input_empty, chrome) {
        return None;
    }
    resolve_cmd_enter(chrome)
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
pub fn lifecycle_chip_label(index_1based: usize, action: &str) -> String {
    format!("⌘{index_1based}  {action}")
}

/// Chip label for affirm: `⌘↩  Confirm`, `⌘↩  Commit`, or `⌘↩  Create change`.
pub fn affirm_chip_label(affirm: Affirm) -> String {
    format!("⌘↩  {}", affirm.send_text())
}

/// Chip label for decline: `⌘⌫  Reject`.
pub fn decline_chip_label() -> String {
    "⌘⌫  Reject".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_lifecycle_chrome() -> ObviousChrome {
        ObviousChrome {
            lifecycle: vec!["/ds-step".into(), "/ds-spec".into(), "/ds-archive".into()],
            affirm: None,
            decline: false,
        }
    }

    fn sample_gate_chrome() -> ObviousChrome {
        ObviousChrome {
            lifecycle: vec!["/ds-step".into(), "/ds-spec".into()],
            affirm: Some(Affirm::Confirm),
            decline: true,
        }
    }

    // @spec chat/obvious-bubble Lifecycle option formatting: Bare skill name formats with leading slash
    #[test]
    fn bare_skill_name_formats_with_leading_slash() {
        // GIVEN a lifecycle skill name stored without a leading slash
        // WHEN the lifecycle send text is derived
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

    // @spec chat/obvious-bubble Lifecycle option formatting: Already-slashed command is preserved
    #[test]
    fn already_slashed_command_is_preserved() {
        // GIVEN a lifecycle skill name that already begins with `/`
        // WHEN the lifecycle send text is derived
        // THEN the send text equals the stored name
        assert_eq!(
            format_lifecycle_command("/ds-spec").as_deref(),
            Some("/ds-spec")
        );
    }

    // @spec chat/obvious-bubble Chrome visibility: Idle empty composer with chrome shows chrome
    #[test]
    fn idle_empty_composer_with_chrome_shows_chrome() {
        // GIVEN non-empty obvious chrome, empty composer, no main turn
        // WHEN chrome visibility is evaluated
        // THEN the chrome is shown
        let chrome = sample_lifecycle_chrome();
        assert!(chrome_visible(false, true, &chrome));
    }

    // @spec chat/obvious-bubble Chrome visibility: Streaming hides chrome
    #[test]
    fn streaming_hides_chrome() {
        let chrome = sample_lifecycle_chrome();
        assert!(!chrome_visible(true, true, &chrome));
    }

    // @spec chat/obvious-bubble Chrome visibility: Non-empty composer hides chrome
    #[test]
    fn non_empty_composer_hides_chrome() {
        let chrome = sample_lifecycle_chrome();
        assert!(!chrome_visible(false, false, &chrome));
    }

    // @spec chat/obvious-bubble Chrome visibility: Empty chrome is hidden
    #[test]
    fn empty_chrome_is_hidden() {
        let chrome = ObviousChrome::default();
        assert!(chrome_is_empty(&chrome));
        assert!(!chrome_visible(false, true, &chrome));
    }

    // @spec chat/obvious-bubble Chrome visibility: Oneshot pending does not hide chrome when otherwise visible
    #[test]
    fn oneshot_pending_does_not_hide_chrome_when_otherwise_visible() {
        // Oneshot state is not a visibility parameter — idle/empty/chrome gates only.
        let chrome = sample_lifecycle_chrome();
        assert!(chrome_visible(false, true, &chrome));
    }

    // @spec chat/obvious-bubble Key resolution: Cmd-Enter sends affirm when present
    #[test]
    fn cmd_enter_sends_affirm_when_present() {
        let chrome = sample_gate_chrome();
        assert!(chrome_visible(false, true, &chrome));
        let sent = resolve_cmd_enter_when_visible(false, true, &chrome).expect("visible");
        assert_eq!(sent, "Confirm");
        assert!(!chrome.lifecycle.contains(&sent));
    }

    // @spec chat/obvious-bubble Key resolution: Cmd-Enter sends first lifecycle when affirm absent
    #[test]
    fn cmd_enter_sends_first_lifecycle_when_affirm_absent() {
        let chrome = sample_lifecycle_chrome();
        let sent = resolve_cmd_enter_when_visible(false, true, &chrome).expect("visible");
        assert_eq!(sent, "/ds-step");
    }

    // @spec chat/obvious-bubble Key resolution: Cmd-Backspace sends Reject when decline set
    #[test]
    fn cmd_backspace_sends_reject_when_decline_set() {
        let chrome = sample_gate_chrome();
        let sent = resolve_cmd_backspace_when_visible(false, true, &chrome).expect("visible");
        assert_eq!(sent, "Reject");
    }

    // @spec chat/obvious-bubble Key resolution: Cmd-digit sends matching lifecycle option
    #[test]
    fn cmd_digit_sends_matching_lifecycle_option() {
        let chrome = sample_lifecycle_chrome();
        let sent = resolve_cmd_digit_when_visible(false, true, &chrome, 2).expect("visible");
        assert_eq!(sent, "/ds-spec");
    }

    // @spec chat/obvious-bubble Key resolution: Resolution is a no-op when chrome not visible
    #[test]
    fn resolution_is_a_no_op_when_chrome_not_visible() {
        let chrome = sample_gate_chrome();
        assert_eq!(
            resolve_cmd_enter_when_visible(true, true, &chrome),
            None
        );
        assert_eq!(
            resolve_cmd_backspace_when_visible(true, true, &chrome),
            None
        );
        assert_eq!(
            resolve_cmd_digit_when_visible(true, true, &chrome, 1),
            None
        );
        assert_eq!(
            resolve_cmd_enter_when_visible(false, false, &chrome),
            None
        );
        let empty = ObviousChrome::default();
        assert_eq!(
            resolve_cmd_enter_when_visible(false, true, &empty),
            None
        );
    }

    // @spec chat/obvious-bubble Key resolution: Resolved text ignores oneshot list when both differ
    #[test]
    fn resolved_text_ignores_oneshot_list_when_both_differ() {
        // Resolution only reads chrome — oneshot list is not a parameter.
        let chrome = ObviousChrome {
            lifecycle: vec!["/ds-archive".into()],
            affirm: None,
            decline: false,
        };
        let oneshot_active = "/ds-review";
        let sent = resolve_cmd_enter_when_visible(false, true, &chrome).expect("visible");
        assert_eq!(sent, "/ds-archive");
        assert_ne!(sent, oneshot_active);
    }

    // @spec chat/obvious-bubble Chip display: Lifecycle chip label is hotkey then action
    #[test]
    fn lifecycle_chip_label_is_hotkey_then_action() {
        let action = "/ds-step";
        let label = lifecycle_chip_label(1, action);
        assert!(label.starts_with("⌘1"), "label={label}");
        assert!(label.contains(action), "label={label}");
        // Send text is the action string only — not the label.
        assert_eq!(action, "/ds-step");
        assert_ne!(label, action);
    }

    // @spec chat/obvious-bubble Chip display: Affirm chip label is hotkey then Confirm, Commit, or Create change
    #[test]
    fn affirm_chip_label_is_hotkey_then_confirm_commit_or_create_change() {
        // GIVEN affirm Create change
        // WHEN the chip label is derived
        // THEN the label starts with the ⌘↩ hotkey
        // AND the label includes `Create change`
        // AND the send text is exactly `Create change`
        let label = affirm_chip_label(Affirm::CreateChange);
        assert!(label.starts_with("⌘↩"), "label={label}");
        assert!(label.contains("Create change"), "label={label}");
        assert_eq!(Affirm::CreateChange.send_text(), "Create change");
        assert_ne!(label, "Create change");

        // Confirm and Commit keep the same hotkey-then-action shape.
        let confirm = affirm_chip_label(Affirm::Confirm);
        assert!(confirm.starts_with("⌘↩"), "label={confirm}");
        assert!(confirm.contains("Confirm"), "label={confirm}");
        assert_eq!(Affirm::Confirm.send_text(), "Confirm");

        let commit = affirm_chip_label(Affirm::Commit);
        assert!(commit.starts_with("⌘↩"), "label={commit}");
        assert!(commit.contains("Commit"), "label={commit}");
        assert_eq!(Affirm::Commit.send_text(), "Commit");

        let decline = decline_chip_label();
        assert!(decline.starts_with("⌘⌫"), "label={decline}");
        assert!(decline.contains("Reject"), "label={decline}");
    }

    // @spec chat/obvious-bubble Ephemeral chrome: Visible chrome is not a stored user message
    #[test]
    fn visible_chrome_is_not_a_stored_user_message() {
        // Chrome helpers only derive visibility/send text — they never append
        // ChatMessage rows. A fresh session stays free of chrome user messages.
        use crate::chat_store::{ChatSession, ContentBlock, Role};

        let chrome = sample_lifecycle_chrome();
        assert!(chrome_visible(false, true, &chrome));
        let action = resolve_cmd_enter(&chrome).expect("lifecycle");

        let session = ChatSession::new("change".into());
        let has_chrome_msg = session.messages.iter().any(|m| {
            matches!(m.role, Role::User)
                && m.content.iter().any(|b| match b {
                    ContentBlock::Text(t) => t == &action || t == &lifecycle_chip_label(1, &action),
                    _ => false,
                })
        });
        assert!(
            !has_chrome_msg,
            "chrome must not appear in the transcript until activation"
        );
    }
}
