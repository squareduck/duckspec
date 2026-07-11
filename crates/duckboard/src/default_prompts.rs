//! Pure helpers for empty-composer next actions and reply-suggestion oneshot.
//!
//! Next actions: empty-session lifecycle bootstrap or trailing `next` meta card.
//! Oneshot: settled parse under the input (agent input hints gated) — separate
//! from next actions. Empty Enter / Tab own next actions only.

use crate::chat_store::{ChatMessage, ChatSession, ContentBlock, Role};
use crate::meta_card::{NextAction, trailing_next_actions};

/// Build the send-form heuristic entry (leading `/`), or empty if none.
///
/// Shares empty-send formatting with [`crate::obvious_bubble::bubble_send_text`].
pub fn heuristic_as_prompts(obvious_command: Option<&str>) -> Vec<String> {
    match crate::obvious_bubble::bubble_send_text(obvious_command) {
        Some(text) => vec![text],
        None => Vec::new(),
    }
}

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
/// - Empty session: first lifecycle option in empty-send form (0 or 1 entry).
/// - Non-empty: trailing next actions from the last assistant message only.
/// - Never merges oneshot results or post-first-turn disk lifecycle options.
pub fn next_action_list(
    session_empty: bool,
    bootstrap: Option<&str>,
    last_assistant: Option<&str>,
) -> Vec<NextAction> {
    if session_empty {
        return match crate::obvious_bubble::bubble_send_text(bootstrap) {
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

/// Oneshot under-input list when agent input hints is on; otherwise empty.
/// At most one entry is kept for display (oneshot is single-suggestion).
pub fn oneshot_display_prompts(
    oneshot_replies: &[String],
    agent_input_hints: bool,
) -> Vec<String> {
    if !agent_input_hints {
        return Vec::new();
    }
    let mut list = oneshot_replies_trimmed(oneshot_replies);
    list.truncate(1);
    list
}

/// Marker shown before an armed under-input oneshot suggestion (`⌘↩`).
pub const ONESHOT_CMD_ENTER_MARKER: &str = "⌘↩";

/// Text to send on empty-composer Cmd-Enter from the armed oneshot suggestion.
///
/// Requires ready (not pending), not streaming, agent input hints on, and a
/// non-empty first oneshot entry. Empty Enter never uses this path.
pub fn oneshot_cmd_submit_text(
    pending: bool,
    is_streaming: bool,
    agent_input_hints: bool,
    oneshot_prompts: &[String],
) -> Option<String> {
    if pending || is_streaming || !agent_input_hints {
        return None;
    }
    oneshot_prompts
        .first()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Whether a reply-suggestion oneshot may start after a turn.
///
/// Requires agent input hints enabled, a non-priming turn, and a non-empty last
/// assistant message (caller has already resolved assistant text).
pub fn should_begin_reply_oneshot(
    agent_input_hints: bool,
    was_priming: bool,
    has_assistant_text: bool,
) -> bool {
    agent_input_hints && !was_priming && has_assistant_text
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

/// Empty-composer oneshot under-input chrome: nothing, loading while pending, or the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefaultsChrome {
    /// Input non-empty, streaming, or ready with an empty oneshot list.
    Hidden,
    /// Oneshot in flight; show loading, not a suggestion.
    Loading,
    /// Ready with a non-empty oneshot list.
    List,
}

/// What the under-input oneshot chrome should show.
///
/// While a main agent turn is streaming, chrome is always hidden (no list and
/// no oneshot loading strip), even if a non-empty list or a pending oneshot
/// would apply when idle.
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
        let got = next_action_list(true, Some("/ds-explore"), None);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].send, "/ds-explore");
        // Bare name formats with leading slash.
        let got = next_action_list(true, Some("ds-propose"), None);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].send, "/ds-propose");
    }

    // @spec chat/default-prompts Next-action list: Empty session without lifecycle yields empty
    #[test]
    fn empty_session_without_lifecycle_yields_empty() {
        // GIVEN empty transcript + no first lifecycle option
        let got = next_action_list(true, None, None);
        assert!(got.is_empty());
        let got = next_action_list(true, Some("  "), None);
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
        let got = next_action_list(false, Some("/ds-explore"), Some(assistant));
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
        let got = next_action_list(false, Some("/ds-explore"), Some(assistant));
        assert!(got.is_empty());
    }

    // @spec chat/default-prompts Next-action list: Oneshot results do not enter the next-action list
    #[test]
    fn oneshot_results_do_not_enter_the_next_action_list() {
        // GIVEN non-empty session + no trailing next + settled oneshot suggestion
        // next_action_list never takes oneshot input — only assistant + bootstrap.
        let assistant = "No trailing next here.";
        let got = next_action_list(false, None, Some(assistant));
        assert!(got.is_empty());
        // Oneshot display is a separate path.
        let oneshot = oneshot_display_prompts(&["nice reply".into()], true);
        assert_eq!(oneshot, vec!["nice reply"]);
        // Merging is not part of next_action_list.
        assert!(got.is_empty());
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
        let actions = vec![
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
        let multi = vec![
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
        assert!(!should_begin_reply_oneshot(false, false, true));
        // Enabled + non-priming + assistant → may start
        assert!(should_begin_reply_oneshot(true, false, true));
        // Priming or missing assistant still blocks
        assert!(!should_begin_reply_oneshot(true, true, true));
        assert!(!should_begin_reply_oneshot(true, false, false));
    }

    // @spec chat/default-prompts Oneshot readiness: Pending hides oneshot row and shows loading
    #[test]
    fn pending_hides_oneshot_row_and_shows_loading() {
        // GIVEN pending oneshot + empty input
        assert_eq!(
            defaults_chrome(true, true, false, 1),
            DefaultsChrome::Loading
        );
        // Typing: no chrome.
        assert_eq!(
            defaults_chrome(false, true, false, 1),
            DefaultsChrome::Hidden
        );
    }

    // @spec chat/default-prompts Oneshot readiness: Empty Cmd-Enter is a no-op while oneshot pending
    #[test]
    fn empty_cmd_enter_is_a_no_op_while_oneshot_pending() {
        // GIVEN pending oneshot + empty composer + a suggestion that would arm when ready
        let prompts = vec!["nice reply".into()];
        assert_eq!(
            oneshot_cmd_submit_text(true, false, true, &prompts),
            None
        );
    }

    // @spec chat/default-prompts Oneshot readiness: Ready after settle arms the oneshot row
    #[test]
    fn ready_after_settle_arms_the_oneshot_row() {
        let list = apply_oneshot_if_current(3, 3, Ok(vec!["nice reply".into()]))
            .expect("matching gen applies");
        let pending = false;
        assert_eq!(list, vec!["nice reply"]);
        assert_eq!(
            defaults_chrome(true, pending, false, list.len()),
            DefaultsChrome::List
        );
        assert_eq!(
            oneshot_cmd_submit_text(pending, false, true, &list).as_deref(),
            Some("nice reply")
        );
    }

    // @spec chat/default-prompts Oneshot readiness: Superseded generation does not arm oneshot
    #[test]
    fn superseded_generation_does_not_arm_oneshot() {
        let applied = apply_oneshot_if_current(5, 4, Ok(vec!["stale".into()]));
        assert!(applied.is_none(), "superseded gen must not replace the list");
    }

    // @spec chat/default-prompts Oneshot readiness: Main turn in progress hides oneshot chrome
    #[test]
    fn main_turn_in_progress_hides_oneshot_chrome() {
        // GIVEN streaming main turn + empty composer + non-empty oneshot
        assert_eq!(
            defaults_chrome(true, false, true, 1),
            DefaultsChrome::Hidden
        );
        assert_eq!(
            defaults_chrome(true, true, true, 1),
            DefaultsChrome::Hidden
        );
        let prompts = vec!["nice reply".into()];
        assert_eq!(
            oneshot_cmd_submit_text(false, true, true, &prompts),
            None
        );
    }

    // @spec chat/default-prompts Oneshot readiness: Timed-out or failed oneshot settles to ready empty
    #[test]
    fn timed_out_or_failed_oneshot_settles_to_ready_empty() {
        let list = apply_oneshot_if_current(
            1,
            1,
            Err("oneshot timed out: oneshot call exceeded budget".into()),
        )
        .expect("matching gen applies");
        let pending = false;
        assert!(list.is_empty());
        assert_eq!(
            defaults_chrome(true, pending, false, list.len()),
            DefaultsChrome::Hidden
        );
        assert_eq!(
            oneshot_cmd_submit_text(pending, false, true, &list),
            None
        );
    }

    // @spec chat/default-prompts Oneshot readiness: Agent handle ends while oneshot pending becomes ready
    #[test]
    fn agent_handle_ends_while_oneshot_pending_becomes_ready() {
        // GIVEN pending oneshot + empty composer → loading.
        assert_eq!(
            defaults_chrome(true, true, false, 0),
            DefaultsChrome::Loading
        );
        // WHEN handle ends without settle → ready, empty, no loading.
        let pending = false;
        let prompts: Vec<String> = Vec::new();
        assert_eq!(
            defaults_chrome(true, pending, false, prompts.len()),
            DefaultsChrome::Hidden
        );
    }

    // @spec chat/default-prompts Oneshot empty-input send: Empty Cmd-Enter sends the armed oneshot suggestion
    #[test]
    fn empty_cmd_enter_sends_the_armed_oneshot_suggestion() {
        let prompts = vec!["sounds good, go ahead".into()];
        assert_eq!(
            oneshot_cmd_submit_text(false, false, true, &prompts).as_deref(),
            Some("sounds good, go ahead")
        );
    }

    // @spec chat/default-prompts Oneshot empty-input send: Empty Cmd-Enter is a no-op when no oneshot suggestion
    #[test]
    fn empty_cmd_enter_is_a_no_op_when_no_oneshot_suggestion() {
        assert_eq!(
            oneshot_cmd_submit_text(false, false, true, &[]),
            None
        );
        // Agent input hints off also disarms.
        let prompts = vec!["would not send".into()];
        assert_eq!(
            oneshot_cmd_submit_text(false, false, false, &prompts),
            None
        );
    }

    // @spec chat/default-prompts Oneshot empty-input send: Empty Enter does not send the oneshot suggestion
    #[test]
    fn empty_enter_does_not_send_the_oneshot_suggestion() {
        // GIVEN empty next-action list + armed oneshot — empty Enter is no-op.
        assert_eq!(next_empty_submit_text(false, &[], 0), None);
        // Oneshot is only on the Cmd-Enter path (binding implemented in TextEdit/app).
        let prompts = vec!["oneshot only".into()];
        assert_eq!(
            oneshot_cmd_submit_text(false, false, true, &prompts).as_deref(),
            Some("oneshot only")
        );
    }

    // @spec chat/default-prompts Oneshot presentation: Armed oneshot shows a Cmd-Enter marker before the suggestion
    #[test]
    fn armed_oneshot_shows_a_cmd_enter_marker_before_the_suggestion() {
        // GIVEN ready non-empty oneshot + empty input
        assert_eq!(
            defaults_chrome(true, false, false, 1),
            DefaultsChrome::List
        );
        assert_eq!(ONESHOT_CMD_ENTER_MARKER, "⌘↩");
        // Display path keeps at most one suggestion.
        let display = oneshot_display_prompts(&["a".into(), "b".into()], true);
        assert_eq!(display, vec!["a"]);
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
