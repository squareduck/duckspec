//! Pure helpers for empty-composer next actions and reply-suggestion oneshot.
//!
//! Next actions: empty-session inherited list or lifecycle bootstrap, else
//! trailing `next` meta card. Oneshot: settled parse (agent input hints gated)
//! may fill fast-response chips when eligible — separate from next actions.
//! Empty Enter / Tab own next actions only. No under-input oneshot chrome.

use crate::chat_store::{ChatMessage, ChatSession, ContentBlock, Role};
use crate::meta_card::{NextAction, trailing_next_actions};

/// Hard cap on oneshot display / chip options (matches duckchat parse cap).
const ONESHOT_DISPLAY_CAP: usize = 3;

/// Trim and drop empties from a oneshot parse (order preserved).
pub fn oneshot_replies_trimmed(oneshot_replies: &[String]) -> Vec<String> {
    oneshot_replies
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Build the next-action list for the empty composer.
///
/// - Empty session + non-empty inherited: that list (outranks bootstrap).
/// - Empty session otherwise: first lifecycle option in empty-send form (0 or 1).
/// - Non-empty: trailing next actions from the last assistant message only
///   (inherited is ignored).
/// - Never merges oneshot results or post-first-turn disk lifecycle options.
pub fn next_action_list(
    session_empty: bool,
    bootstrap: Option<&str>,
    last_assistant: Option<&str>,
    inherited: Option<&[NextAction]>,
) -> Vec<NextAction> {
    if session_empty {
        if let Some(inh) = inherited {
            if !inh.is_empty() {
                return inh.to_vec();
            }
        }
        return match crate::fast_response::lifecycle_send_text(bootstrap) {
            Some(send) => vec![NextAction {
                send,
                reason: None,
            }],
            None => Vec::new(),
        };
    }
    match last_assistant {
        Some(src) if !src.trim().is_empty() => trailing_next_actions(src),
        _ => Vec::new(),
    }
}

/// Settled oneshot list when agent input hints is on; otherwise empty.
/// At most three entries are kept (parser may already cap).
pub fn oneshot_display_prompts(
    oneshot_replies: &[String],
    agent_input_hints: bool,
) -> Vec<String> {
    if !agent_input_hints {
        return Vec::new();
    }
    let mut list = oneshot_replies_trimmed(oneshot_replies);
    list.truncate(ONESHOT_DISPLAY_CAP);
    list
}

/// Whether a reply-suggestion oneshot may start after a turn.
///
/// Requires agent input hints enabled, a non-priming turn, non-empty last
/// assistant text (caller resolved), and an empty next-action list (ghost
/// would win — skip the model call).
pub fn should_begin_reply_oneshot(
    agent_input_hints: bool,
    was_priming: bool,
    has_assistant_text: bool,
    next_actions_empty: bool,
) -> bool {
    agent_input_hints && !was_priming && has_assistant_text && next_actions_empty
}

/// Whether settled oneshot replies may occupy the fast-response shell.
pub fn oneshot_chips_allowed(
    is_streaming: bool,
    is_awaiting_user: bool,
    next_actions_len: usize,
    agent_input_hints: bool,
    oneshot_len: usize,
) -> bool {
    agent_input_hints
        && !is_streaming
        && !is_awaiting_user
        && next_actions_len == 0
        && oneshot_len > 0
}

/// Under-input oneshot chrome was removed — never shown (no loading strip, no
/// suggestion row under the composer).
#[cfg(test)]
fn oneshot_under_input_chrome_visible(
    _input_empty: bool,
    _pending: bool,
    _is_streaming: bool,
    _prompts_len: usize,
) -> bool {
    false
}

/// Text to send on empty-composer Enter from the next-action list.
///
/// Not gated on oneshot pending — next actions stay armed while a
/// reply-suggestion oneshot is in flight. Blocked only while the main turn
/// is streaming.
pub fn next_empty_submit_text(
    is_streaming: bool,
    actions: &[NextAction],
    active_idx: usize,
) -> Option<String> {
    if is_streaming {
        return None;
    }
    actions.get(active_idx).map(|a| a.send.clone())
}

/// Whether Tab/Shift-Tab may cycle next actions (idle, empty input, ≥2 actions).
pub fn can_cycle_next_actions(is_streaming: bool, actions_len: usize) -> bool {
    !is_streaming && actions_len >= 2
}

/// Ghost (placeholder) text for the empty composer when a next action is armed.
/// Hidden while the main turn is streaming so the previous next token does not
/// linger after send.
pub fn next_ghost_text(
    is_streaming: bool,
    actions: &[NextAction],
    active_idx: usize,
) -> Option<&str> {
    if is_streaming {
        return None;
    }
    actions.get(active_idx).map(|a| a.send.as_str())
}

/// Whether to show the tab-available marker for multi next actions.
pub fn next_tab_marker_visible(
    input_empty: bool,
    is_streaming: bool,
    actions_len: usize,
) -> bool {
    input_empty && !is_streaming && actions_len > 1
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
        Ok(list) => oneshot_replies_trimmed(&list),
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

/// Active next-action index after rebuilding the list.
///
/// - `after_turn`: always `0` so the ghost matches the first ranked action on
///   the card the user just saw.
/// - Otherwise: `0` when ordered send tokens changed; keep `prev_idx` (clamped)
///   when the list is unchanged so Tab cycle survives chrome/scope rebuilds.
pub fn next_action_idx_after_refresh(
    after_turn: bool,
    prev_sends: &[&str],
    new_sends: &[&str],
    prev_idx: usize,
) -> usize {
    if after_turn || prev_sends != new_sends {
        0
    } else {
        clamp_active_index(new_sends.len(), prev_idx)
    }
}

/// Last non-priming assistant text and the preceding non-priming user text, if
/// any. Used to build the reply-suggestion oneshot request and next-action
/// trailing parse.
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

    // @spec chat/default-prompts Next-action list: Empty session seeds first lifecycle
    #[test]
    fn empty_session_seeds_first_lifecycle() {
        // GIVEN empty transcript + first lifecycle option in empty-send form
        let got = next_action_list(true, Some("/ds-explore"), None, None);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].send, "/ds-explore");
        // Bare name formats with leading slash.
        let got = next_action_list(true, Some("ds-propose"), None, None);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].send, "/ds-propose");
    }

    // @spec chat/default-prompts Next-action list: Empty session without lifecycle yields empty
    #[test]
    fn empty_session_without_lifecycle_yields_empty() {
        // GIVEN empty transcript + no first lifecycle option
        let got = next_action_list(true, None, None, None);
        assert!(got.is_empty());
        let got = next_action_list(true, Some("  "), None, None);
        assert!(got.is_empty());
    }

    // @spec chat/default-prompts Next-action list: Non-empty session uses trailing next actions only
    #[test]
    fn non_empty_session_uses_trailing_next_actions_only() {
        // GIVEN non-empty session + trailing next with two send tokens + different lifecycle
        let assistant = "\
Done.

> **next**
>
> `/ds-spec`  write specs
> `/ds-design`  design it
";
        let got = next_action_list(false, Some("/ds-explore"), Some(assistant), None);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].send, "/ds-spec");
        assert_eq!(got[1].send, "/ds-design");
        // Lifecycle bootstrap must not appear.
        assert!(!got.iter().any(|a| a.send == "/ds-explore"));
    }

    // @spec chat/default-prompts Next-action list: Non-empty session without trailing next yields empty
    #[test]
    fn non_empty_session_without_trailing_next_yields_empty() {
        // GIVEN non-empty session + assistant without trailing next + lifecycle present
        let assistant = "Just some prose, no meta card.";
        let got = next_action_list(false, Some("/ds-explore"), Some(assistant), None);
        assert!(got.is_empty());
    }

    // @spec chat/default-prompts Next-action list: Oneshot results do not enter the next-action list
    #[test]
    fn oneshot_results_do_not_enter_the_next_action_list() {
        // GIVEN non-empty session + no trailing next + settled oneshot suggestion
        // next_action_list never takes oneshot input — only assistant + bootstrap.
        let assistant = "No trailing next here.";
        let got = next_action_list(false, None, Some(assistant), None);
        assert!(got.is_empty());
        // Oneshot display is a separate path.
        let oneshot = oneshot_display_prompts(&["nice reply".into()], true);
        assert_eq!(oneshot, vec!["nice reply"]);
        // Merging is not part of next_action_list.
        assert!(got.is_empty());
    }

    // @spec chat/default-prompts Next-action list: Empty session with inherited next actions uses inherited list
    #[test]
    fn empty_session_with_inherited_next_actions_uses_inherited_list() {
        // GIVEN empty transcript + two inherited send tokens + different lifecycle
        let inherited = [
            NextAction {
                send: "/ds-spec".into(),
                reason: Some("write specs".into()),
            },
            NextAction {
                send: "confirm".into(),
                reason: None,
            },
        ];
        let got = next_action_list(
            true,
            Some("/ds-explore"),
            None,
            Some(&inherited),
        );
        // THEN exactly the inherited tokens in order (not lifecycle)
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].send, "/ds-spec");
        assert_eq!(got[1].send, "confirm");
        assert!(!got.iter().any(|a| a.send == "/ds-explore"));
    }

    // @spec chat/default-prompts Next-action list: Empty session without inherited falls back to lifecycle
    #[test]
    fn empty_session_without_inherited_falls_back_to_lifecycle() {
        // GIVEN empty transcript + no non-empty inherited + lifecycle option
        let got = next_action_list(true, Some("/ds-propose"), None, None);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].send, "/ds-propose");
        // Empty inherited slice is treated as absent.
        let empty: &[NextAction] = &[];
        let got = next_action_list(true, Some("ds-apply"), None, Some(empty));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].send, "/ds-apply");
    }

    // @spec chat/default-prompts Next-action list: Non-empty session drops inheritance
    #[test]
    fn non_empty_session_drops_inheritance() {
        // GIVEN non-empty session + inherited list + trailing next with different token
        let inherited = [NextAction {
            send: "confirm".into(),
            reason: None,
        }];
        let assistant = "\
Done.

> **next**
>
> `/ds-step`  plan implementation
";
        let got = next_action_list(
            false,
            Some("/ds-explore"),
            Some(assistant),
            Some(&inherited),
        );
        // THEN trailing next only; inherited not used
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].send, "/ds-step");
        assert!(!got.iter().any(|a| a.send == "confirm"));
    }

    // @spec chat/default-prompts Next-action empty-input send and cycle: Empty submit sends the active next action
    #[test]
    fn empty_submit_sends_the_active_next_action() {
        // GIVEN empty composer, non-empty next-action list, active index 1
        let actions = next_action_list(
            false,
            None,
            Some(
                "\
> **next**
>
> `/ds-spec`
> `/ds-design`
",
            ),
            None,
        );
        assert_eq!(
            next_empty_submit_text(false, &actions, 1).as_deref(),
            Some("/ds-design")
        );
    }

    // @spec chat/default-prompts Next-action empty-input send and cycle: Empty submit is a no-op when the next-action list is empty
    #[test]
    fn empty_submit_is_a_no_op_when_the_next_action_list_is_empty() {
        // GIVEN empty composer + empty next-action list
        assert_eq!(next_empty_submit_text(false, &[], 0), None);
    }

    #[test]
    fn next_action_idx_resets_after_turn_not_on_noop_rebuild() {
        // After a turn: always first ranked action.
        assert_eq!(
            next_action_idx_after_refresh(true, &["a", "b"], &["a", "b"], 1),
            0
        );
        // New list (any non-turn rebuild): reset.
        assert_eq!(
            next_action_idx_after_refresh(false, &["a", "b", "c"], &["confirm", "reject"], 2),
            0
        );
        // Same list, chrome rebuild: preserve Tab index.
        assert_eq!(
            next_action_idx_after_refresh(false, &["a", "b", "c"], &["a", "b", "c"], 2),
            2
        );
    }

    // @spec chat/default-prompts Next-action empty-input send and cycle: Tab cycles next actions with wrap
    #[test]
    fn tab_cycles_next_actions_with_wrap() {
        // GIVEN empty composer, ≥2 next actions, Tab at last index
        let actions = [
            NextAction {
                send: "a".into(),
                reason: None,
            },
            NextAction {
                send: "b".into(),
                reason: None,
            },
            NextAction {
                send: "c".into(),
                reason: None,
            },
        ];
        assert!(can_cycle_next_actions(false, actions.len()));
        let last = actions.len() - 1;
        let next = cycle_active_index(actions.len(), last, 1);
        assert_eq!(next, 0);
        assert_eq!(cycle_active_index(actions.len(), 0, -1), last);
        // Composer input remains empty — cycle only mutates the index (caller).
    }

    // @spec chat/default-prompts Next-action empty-input send and cycle: Multi next shows a tab-available marker
    #[test]
    fn multi_next_shows_a_tab_available_marker() {
        // GIVEN empty composer + next-action list of at least two
        assert!(next_tab_marker_visible(true, false, 2));
        assert!(!next_tab_marker_visible(true, false, 1));
        assert!(!next_tab_marker_visible(true, false, 0));
        assert!(!next_tab_marker_visible(false, false, 2));
        assert!(!next_tab_marker_visible(true, true, 2));
        // Ghost text is the active send when idle; tab marker sits before ghost
        // in the view (see agent_chat) when multi next is armed.
        let actions = vec![
            NextAction {
                send: "/ds-spec".into(),
                reason: None,
            },
            NextAction {
                send: "/ds-design".into(),
                reason: None,
            },
        ];
        assert_eq!(next_ghost_text(false, &actions, 0), Some("/ds-spec"));
        assert_eq!(next_ghost_text(false, &actions, 1), Some("/ds-design"));
        // Ghost clears while streaming (after send / mid-turn).
        assert_eq!(next_ghost_text(true, &actions, 0), None);
    }

    // @spec chat/default-prompts Oneshot readiness: Empty Enter still sends next action while oneshot pending
    #[test]
    fn empty_enter_still_sends_next_action_while_oneshot_pending() {
        // GIVEN pending oneshot + empty composer + non-empty next-action list
        let actions = vec![NextAction {
            send: "/ds-spec".into(),
            reason: None,
        }];
        // Pending oneshot does not gate next-action empty Enter.
        assert_eq!(
            next_empty_submit_text(false, &actions, 0).as_deref(),
            Some("/ds-spec")
        );
        // Tab still cycles multi next while pending.
        let multi = [
            NextAction {
                send: "a".into(),
                reason: None,
            },
            NextAction {
                send: "b".into(),
                reason: None,
            },
        ];
        assert!(can_cycle_next_actions(false, multi.len()));
    }

    // @spec chat/default-prompts Agent input hints gate: Oneshot launch requires agent input hints enabled
    #[test]
    fn oneshot_launch_requires_agent_input_hints_enabled() {
        // GIVEN agent input hints disabled + non-priming turn that would qualify
        assert!(!should_begin_reply_oneshot(false, false, true, true));
        // Enabled + non-priming + assistant + empty next-actions → may start
        assert!(should_begin_reply_oneshot(true, false, true, true));
        // Priming or missing assistant still blocks
        assert!(!should_begin_reply_oneshot(true, true, true, true));
        assert!(!should_begin_reply_oneshot(true, false, false, true));
    }

    // @spec chat/default-prompts Agent input hints gate: Oneshot launch is skipped when the next-action list is non-empty
    #[test]
    fn oneshot_launch_is_skipped_when_the_next_action_list_is_non_empty() {
        // GIVEN hints on + non-priming + assistant + non-empty next-actions
        assert!(!should_begin_reply_oneshot(true, false, true, false));
        // Empty next-actions still allows launch when other gates pass
        assert!(should_begin_reply_oneshot(true, false, true, true));
    }

    // @spec chat/default-prompts Oneshot readiness: Superseded generation does not arm oneshot
    #[test]
    fn superseded_generation_does_not_arm_oneshot() {
        let applied = apply_oneshot_if_current(5, 4, Ok(vec!["stale".into()]));
        assert!(applied.is_none(), "superseded gen must not replace the list");
    }

    // @spec chat/default-prompts Oneshot readiness: Failed or timed-out oneshot settles without presenting suggestions
    #[test]
    fn failed_or_timed_out_oneshot_settles_without_presenting_suggestions() {
        let list = apply_oneshot_if_current(
            1,
            1,
            Err("oneshot timed out: oneshot call exceeded budget".into()),
        )
        .expect("matching gen applies");
        // Ready with empty list — no suggestions to present
        assert!(list.is_empty());
        assert!(!oneshot_chips_allowed(false, false, 0, true, list.len()));
        assert!(!oneshot_under_input_chrome_visible(true, false, false, list.len()));
    }

    // @spec chat/default-prompts Oneshot readiness: Agent handle end while pending leaves suggestions ready empty
    #[test]
    fn agent_handle_end_while_pending_leaves_suggestions_ready_empty() {
        // WHEN handle ends without settle → ready, empty list, no loading chrome
        let pending = false;
        let prompts: Vec<String> = Vec::new();
        assert!(!oneshot_under_input_chrome_visible(
            true,
            pending,
            false,
            prompts.len()
        ));
        assert!(!oneshot_chips_allowed(false, false, 0, true, prompts.len()));
    }

    // @spec chat/default-prompts Oneshot readiness: Pending oneshot presents no loading chrome
    #[test]
    fn pending_oneshot_presents_no_loading_chrome() {
        // GIVEN pending oneshot + empty composer — under-input chrome never shows
        assert!(!oneshot_under_input_chrome_visible(true, true, false, 0));
        assert!(!oneshot_under_input_chrome_visible(true, true, false, 1));
        // Pending also has no settled list for chips yet
        assert!(!oneshot_chips_allowed(false, false, 0, true, 0));
    }

    // @spec chat/default-prompts Oneshot chip eligibility: Eligible when idle with no next actions and a settled list
    #[test]
    fn eligible_when_idle_with_no_next_actions_and_a_settled_list() {
        assert!(oneshot_chips_allowed(false, false, 0, true, 2));
    }

    // @spec chat/default-prompts Oneshot chip eligibility: Ineligible when next-action list is non-empty
    #[test]
    fn ineligible_when_next_action_list_is_non_empty() {
        assert!(!oneshot_chips_allowed(false, false, 1, true, 2));
    }

    // @spec chat/default-prompts Oneshot chip eligibility: Ineligible while awaiting a user choice
    #[test]
    fn ineligible_while_awaiting_a_user_choice() {
        assert!(!oneshot_chips_allowed(false, true, 0, true, 2));
    }

    // @spec chat/default-prompts Oneshot chip eligibility: Ineligible while streaming
    #[test]
    fn ineligible_while_streaming() {
        assert!(!oneshot_chips_allowed(true, false, 0, true, 2));
    }

    // @spec chat/default-prompts Oneshot chip eligibility: Ineligible when the settled list is empty
    #[test]
    fn ineligible_when_the_settled_list_is_empty() {
        assert!(!oneshot_chips_allowed(false, false, 0, true, 0));
    }

    #[test]
    fn oneshot_display_keeps_up_to_three() {
        let display = oneshot_display_prompts(
            &["a".into(), "b".into(), "c".into(), "d".into()],
            true,
        );
        assert_eq!(display, vec!["a", "b", "c"]);
        assert!(oneshot_display_prompts(&["a".into()], false).is_empty());
    }

    // Empty Enter never sends oneshot — next-action path only.
    #[test]
    fn empty_enter_does_not_send_oneshot_when_next_actions_empty() {
        assert_eq!(next_empty_submit_text(false, &[], 0), None);
    }
}
