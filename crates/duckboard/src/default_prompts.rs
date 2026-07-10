//! Pure helpers for conversation-local empty-input default prompts.
//!
//! The effective list is only a settled non-empty oneshot parse. The lifecycle
//! heuristic is not a list entry (it remains a soft oneshot request hint and
//! empty-send formatting helper for the obvious bubble). Helpers drive empty-
//! submit / Tab-cycle selection without touching the composer buffer, gated on
//! oneshot readiness.

use crate::chat_store::{ChatMessage, ChatSession, ContentBlock, Role};

/// Build the send-form heuristic entry (leading `/`), or empty if none.
///
/// Shares empty-send formatting with [`crate::obvious_bubble::bubble_send_text`].
/// Used for oneshot soft-hint display and any remaining send-form callers — **not**
/// for the composer default-prompt list.
pub fn heuristic_as_prompts(obvious_command: Option<&str>) -> Vec<String> {
    match crate::obvious_bubble::bubble_send_text(obvious_command) {
        Some(text) => vec![text],
        None => Vec::new(),
    }
}

/// Effective empty-composer defaults: non-empty oneshot parse only (order
/// preserved). Empty when no non-empty parse is armed.
pub fn effective_prompts(oneshot_replies: &[String]) -> Vec<String> {
    oneshot_replies
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Text to send on empty-composer submit when suggestions are ready, or
/// `None` when pending, streaming, the list is empty, or the index is out of range.
pub fn empty_submit_text(
    pending: bool,
    is_streaming: bool,
    prompts: &[String],
    active_idx: usize,
) -> Option<String> {
    if pending || is_streaming {
        return None;
    }
    prompts.get(active_idx).cloned()
}

/// Whether Tab/Shift-Tab may cycle the active default (ready + non-empty list,
/// not pending, not mid-turn).
pub fn can_cycle_defaults(pending: bool, is_streaming: bool, prompts_len: usize) -> bool {
    !pending && !is_streaming && prompts_len > 0
}

/// Apply a oneshot result only when its generation still matches the session.
/// Returns `None` when superseded (caller leaves list and readiness unchanged).
/// On match: non-empty parse wins; empty parse or error yields an empty list.
pub fn apply_oneshot_if_current(
    session_gen: u64,
    result_gen: u64,
    result: Result<Vec<String>, String>,
) -> Option<Vec<String>> {
    if result_gen != session_gen {
        return None;
    }
    Some(match result {
        Ok(list) => effective_prompts(&list),
        Err(_) => Vec::new(),
    })
}

/// Advance or reverse the active index with wrap. `delta` is typically `+1`
/// (Tab) or `-1` (Shift-Tab). Empty list returns `0`.
pub fn cycle_active_index(len: usize, active_idx: usize, delta: i8) -> usize {
    if len == 0 {
        return 0;
    }
    let len_i = len as isize;
    let cur = (active_idx % len) as isize;
    let next = (cur + delta as isize).rem_euclid(len_i);
    next as usize
}

/// Clamp `active_idx` into `[0, len)` (or `0` when empty) after the list changes.
pub fn clamp_active_index(len: usize, active_idx: usize) -> usize {
    if len == 0 {
        0
    } else {
        active_idx.min(len - 1)
    }
}

/// Empty-composer defaults chrome: nothing, loading while pending, or the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefaultsChrome {
    /// Input non-empty, or ready with an empty list.
    Hidden,
    /// Oneshot in flight; show loading, not a prompt list.
    Loading,
    /// Ready with a non-empty effective list.
    List,
}

/// What the empty-input defaults chrome should show.
///
/// While a main agent turn is streaming, chrome is always hidden (no list and
/// no oneshot loading strip), even if a non-empty effective list or a pending
/// oneshot would apply when idle.
pub fn defaults_chrome(
    input_empty: bool,
    pending: bool,
    is_streaming: bool,
    prompts_len: usize,
) -> DefaultsChrome {
    if !input_empty || is_streaming {
        return DefaultsChrome::Hidden;
    }
    if pending {
        return DefaultsChrome::Loading;
    }
    if prompts_len == 0 {
        DefaultsChrome::Hidden
    } else {
        DefaultsChrome::List
    }
}

/// Last non-priming assistant text and the preceding non-priming user text, if
/// any. Used to build the reply-suggestion oneshot request.
pub fn last_assistant_and_user(session: &ChatSession) -> Option<(String, Option<String>)> {
    let mut last_asst: Option<(usize, String)> = None;
    for (i, msg) in session.messages.iter().enumerate().rev() {
        if msg.is_priming || !matches!(msg.role, Role::Assistant) {
            continue;
        }
        let text = message_plain_text(msg);
        if text.trim().is_empty() {
            continue;
        }
        last_asst = Some((i, text));
        break;
    }
    let (idx, assistant) = last_asst?;
    let mut user = None;
    for msg in session.messages[..idx].iter().rev() {
        if msg.is_priming || !matches!(msg.role, Role::User) {
            continue;
        }
        let text = message_plain_text(msg);
        if !text.trim().is_empty() {
            user = Some(text);
        }
        break;
    }
    Some((assistant, user))
}

fn message_plain_text(msg: &ChatMessage) -> String {
    msg.content
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    // @spec chat/default-prompts Effective default-prompt list: Parsed replies are the effective list in order
    #[test]
    fn parsed_replies_are_the_effective_list_in_order() {
        let agent = vec![
            "/ds-spec".into(),
            "yes, continue".into(),
            "no, skip".into(),
        ];
        let got = effective_prompts(&agent);
        assert_eq!(
            got,
            vec!["/ds-spec", "yes, continue", "no, skip"]
        );
    }

    // @spec chat/default-prompts Effective default-prompt list: No non-empty oneshot result yields an empty list
    #[test]
    fn no_non_empty_oneshot_result_yields_an_empty_list() {
        // GIVEN a session with no settled non-empty oneshot result
        let got = effective_prompts(&[]);
        assert!(got.is_empty());
    }

    // @spec chat/default-prompts Effective default-prompt list: Failed or empty oneshot yields an empty list even with a heuristic
    #[test]
    fn failed_or_empty_oneshot_yields_an_empty_list_even_with_a_heuristic() {
        // Settled empty parse — list empty (heuristic is not a list entry).
        let got = apply_oneshot_if_current(1, 1, Ok(vec![])).expect("matching gen applies");
        assert!(got.is_empty());

        // Settled error — same empty list.
        let got =
            apply_oneshot_if_current(1, 1, Err("boom".into())).expect("matching gen applies");
        assert!(got.is_empty());
    }

    // @spec chat/default-prompts Empty-input send and cycle: Empty submit sends the active prompt
    #[test]
    fn empty_submit_sends_the_active_prompt() {
        // GIVEN empty composer, ready suggestions, non-empty list, active index 1
        let prompts = vec!["/ds-spec".into(), "/ds-design".into()];
        assert_eq!(
            empty_submit_text(false, false, &prompts, 1).as_deref(),
            Some("/ds-design")
        );
    }

    // @spec chat/default-prompts Empty-input send and cycle: Empty submit is a no-op when the list is empty
    #[test]
    fn empty_submit_is_a_no_op_when_the_list_is_empty() {
        // GIVEN empty composer, ready suggestions, empty list
        assert_eq!(empty_submit_text(false, false, &[], 0), None);
    }

    // @spec chat/default-prompts Empty-input send and cycle: Tab cycles active index with wrap
    #[test]
    fn tab_cycles_active_index_with_wrap() {
        // GIVEN empty composer, ready, list of ≥2, Tab at last index
        let prompts: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        assert!(can_cycle_defaults(false, false, prompts.len()));
        let last = prompts.len() - 1;
        let next = cycle_active_index(prompts.len(), last, 1);
        assert_eq!(next, 0);
        assert_eq!(cycle_active_index(prompts.len(), 0, -1), last);
    }

    // @spec chat/default-prompts Suggestion readiness: Empty submit is a no-op while pending
    #[test]
    fn empty_submit_is_a_no_op_while_pending() {
        let prompts = vec!["/ds-spec".into(), "yes".into()];
        assert_eq!(empty_submit_text(true, false, &prompts, 0), None);
        assert!(!can_cycle_defaults(true, false, prompts.len()));
    }

    // @spec chat/default-prompts Suggestion readiness: Ready after settle arms the effective list
    #[test]
    fn ready_after_settle_arms_the_effective_list() {
        let list = apply_oneshot_if_current(
            3,
            3,
            Ok(vec!["/ds-spec".into(), "no thanks".into()]),
        )
        .expect("matching gen applies");
        let pending = false; // settled → ready
        assert_eq!(list, vec!["/ds-spec", "no thanks"]);
        assert_eq!(
            empty_submit_text(pending, false, &list, 0).as_deref(),
            Some("/ds-spec")
        );
        assert!(can_cycle_defaults(pending, false, list.len()));
    }

    // @spec chat/default-prompts Suggestion readiness: Superseded generation does not arm the list
    #[test]
    fn superseded_generation_does_not_arm_the_list() {
        let applied = apply_oneshot_if_current(5, 4, Ok(vec!["stale".into()]));
        assert!(applied.is_none(), "superseded gen must not replace the list");
    }

    // @spec chat/default-prompts Suggestion readiness: Pending hides list and shows loading
    #[test]
    fn pending_hides_list_and_shows_loading() {
        // GIVEN pending oneshot + empty input — even if a list would exist.
        assert_eq!(
            defaults_chrome(true, true, false, 3),
            DefaultsChrome::Loading
        );
        // Typing: no chrome (no height reserve for suggestions).
        assert_eq!(
            defaults_chrome(false, true, false, 3),
            DefaultsChrome::Hidden
        );
        assert_eq!(
            defaults_chrome(false, false, false, 3),
            DefaultsChrome::Hidden
        );
        // Ready + non-empty → list.
        assert_eq!(
            defaults_chrome(true, false, false, 2),
            DefaultsChrome::List
        );
        // Ready + empty → hidden.
        assert_eq!(
            defaults_chrome(true, false, false, 0),
            DefaultsChrome::Hidden
        );
    }

    // @spec chat/default-prompts Suggestion readiness: Main turn in progress hides default prompts
    #[test]
    fn main_turn_in_progress_hides_default_prompts() {
        // GIVEN streaming main turn + empty composer + non-empty effective list
        // would otherwise be available (and even if oneshot is pending).
        assert_eq!(
            defaults_chrome(true, false, true, 3),
            DefaultsChrome::Hidden
        );
        assert_eq!(
            defaults_chrome(true, true, true, 3),
            DefaultsChrome::Hidden
        );
        // Empty Enter / Tab cycle from defaults stay disarmed while streaming.
        let prompts = vec!["/ds-spec".into(), "yes".into()];
        assert_eq!(empty_submit_text(false, true, &prompts, 0), None);
        assert!(!can_cycle_defaults(false, true, prompts.len()));
    }

    // @spec chat/default-prompts Suggestion readiness: Timed-out or failed oneshot settles to ready
    #[test]
    fn timed_out_or_failed_oneshot_settles_to_ready() {
        // GIVEN pending oneshot that settles as failure (timeout-shaped Err) for
        // the current generation + empty composer.
        let list = apply_oneshot_if_current(
            1,
            1,
            Err("oneshot timed out: oneshot call exceeded budget".into()),
        )
        .expect("matching gen applies");
        let pending = false; // settled → ready
        assert!(list.is_empty(), "failure with no parse yields empty list");
        assert_eq!(
            defaults_chrome(true, pending, false, list.len()),
            DefaultsChrome::Hidden
        );
        // Plain failure settles the same way (no loading; empty list).
        let list2 =
            apply_oneshot_if_current(2, 2, Err("boom".into())).expect("matching gen applies");
        assert!(list2.is_empty());
        assert_eq!(
            defaults_chrome(true, false, false, list2.len()),
            DefaultsChrome::Hidden
        );
    }

    // @spec chat/default-prompts Suggestion readiness: Agent handle ends while suggestions pending
    #[test]
    fn agent_handle_ends_while_suggestions_pending() {
        // GIVEN pending oneshot + empty composer → loading.
        assert_eq!(
            defaults_chrome(true, true, false, 0),
            DefaultsChrome::Loading
        );
        // WHEN the chat agent handle ends without a settle (ProcessExited clears
        // pending; effective list stays empty without a parse).
        let pending = false;
        let prompts: Vec<String> = Vec::new();
        // THEN loading is not shown; suggestions are ready (list may be empty).
        assert_eq!(
            defaults_chrome(true, pending, false, prompts.len()),
            DefaultsChrome::Hidden
        );
        assert_eq!(empty_submit_text(pending, false, &prompts, 0), None);
    }

    #[test]
    fn heuristic_as_prompts_adds_leading_slash() {
        assert_eq!(
            heuristic_as_prompts(Some("ds-explore")),
            vec!["/ds-explore"]
        );
        assert_eq!(
            heuristic_as_prompts(Some("/ds-spec")),
            vec!["/ds-spec"]
        );
        assert!(heuristic_as_prompts(None).is_empty());
        assert!(heuristic_as_prompts(Some("  ")).is_empty());
    }
}
