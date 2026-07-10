//! Pure helpers for conversation-local empty-input default prompts.
//!
//! The effective list is the oneshot parse result only (no post-merge of the
//! lifecycle heuristic). Helpers drive empty-submit / Tab-cycle selection
//! without touching the composer buffer, gated on oneshot readiness.

use crate::chat_store::{ChatMessage, ChatSession, ContentBlock, Role};

/// Effective empty-composer defaults: the parsed oneshot suggestion list
/// alone (order preserved). The lifecycle heuristic is never appended here —
/// it is only a soft hint on the oneshot request.
pub fn effective_prompts(agent_replies: &[String]) -> Vec<String> {
    agent_replies
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Text to send on empty-composer submit when suggestions are ready, or
/// `None` when pending, the list is empty, or the index is out of range.
pub fn empty_submit_text(
    pending: bool,
    prompts: &[String],
    active_idx: usize,
) -> Option<String> {
    if pending {
        return None;
    }
    prompts.get(active_idx).cloned()
}

/// Whether Tab/Shift-Tab may cycle the active default (ready + non-empty list).
pub fn can_cycle_defaults(pending: bool, prompts_len: usize) -> bool {
    !pending && prompts_len > 0
}

/// Apply a oneshot result only when its generation still matches the session.
/// Returns `None` when superseded (caller leaves list and readiness unchanged).
/// On match: `Ok(list)` or `Err` both settle to ready — error → empty list.
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
pub fn defaults_chrome(input_empty: bool, pending: bool, prompts_len: usize) -> DefaultsChrome {
    if !input_empty {
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

    // @spec chat/default-prompts Effective list is oneshot result only: Parsed replies are the effective list in order
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

    // @spec chat/default-prompts Effective list is oneshot result only: Failed or empty oneshot yields empty effective list
    #[test]
    fn failed_or_empty_oneshot_yields_empty_effective_list() {
        // Settled empty parse / failed oneshot → empty agent list.
        // Lifecycle heuristic is present but must not be post-merged.
        let _heuristic = Some("ds-design");
        let got = effective_prompts(&[]);
        assert!(got.is_empty());
    }

    // @spec chat/default-prompts Empty-input send and cycle: Empty submit sends the active prompt
    #[test]
    fn empty_submit_sends_the_active_prompt() {
        // GIVEN empty composer, ready suggestions, non-empty list, active index 1
        let prompts = vec!["/ds-spec".into(), "/ds-design".into()];
        assert_eq!(
            empty_submit_text(false, &prompts, 1).as_deref(),
            Some("/ds-design")
        );
    }

    // @spec chat/default-prompts Empty-input send and cycle: Empty submit is a no-op when the list is empty
    #[test]
    fn empty_submit_is_a_no_op_when_the_list_is_empty() {
        // GIVEN empty composer, ready suggestions, empty list
        assert_eq!(empty_submit_text(false, &[], 0), None);
    }

    // @spec chat/default-prompts Empty-input send and cycle: Tab cycles active index with wrap
    #[test]
    fn tab_cycles_active_index_with_wrap() {
        // GIVEN empty composer, ready, list of ≥2, Tab at last index
        let prompts: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        assert!(can_cycle_defaults(false, prompts.len()));
        let last = prompts.len() - 1;
        let next = cycle_active_index(prompts.len(), last, 1);
        assert_eq!(next, 0);
        assert_eq!(cycle_active_index(prompts.len(), 0, -1), last);
    }

    // @spec chat/default-prompts Suggestion readiness: Empty submit is a no-op while pending
    #[test]
    fn empty_submit_is_a_no_op_while_pending() {
        let prompts = vec!["/ds-spec".into(), "yes".into()];
        assert_eq!(empty_submit_text(true, &prompts, 0), None);
        assert!(!can_cycle_defaults(true, prompts.len()));
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
            empty_submit_text(pending, &list, 0).as_deref(),
            Some("/ds-spec")
        );
        assert!(can_cycle_defaults(pending, list.len()));
    }

    // @spec chat/default-prompts Suggestion readiness: Superseded generation does not arm the list
    #[test]
    fn superseded_generation_does_not_arm_the_list() {
        let applied = apply_oneshot_if_current(
            5,
            4,
            Ok(vec!["stale".into()]),
        );
        assert!(applied.is_none(), "superseded gen must not replace the list");
    }

    // @spec chat/default-prompts Suggestion readiness: Pending hides list and shows loading
    #[test]
    fn pending_hides_list_and_shows_loading() {
        // GIVEN pending oneshot + empty input — even if a list would exist.
        assert_eq!(
            defaults_chrome(true, true, 3),
            DefaultsChrome::Loading
        );
        // Typing: no chrome (no height reserve for suggestions).
        assert_eq!(defaults_chrome(false, true, 3), DefaultsChrome::Hidden);
        assert_eq!(defaults_chrome(false, false, 3), DefaultsChrome::Hidden);
        // Ready + non-empty → list.
        assert_eq!(defaults_chrome(true, false, 2), DefaultsChrome::List);
        // Ready + empty → hidden.
        assert_eq!(defaults_chrome(true, false, 0), DefaultsChrome::Hidden);
    }
}
