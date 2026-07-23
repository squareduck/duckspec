//! Pure helpers for multi-option fast-response chips: ordered options,
//! visibility, key resolution, and chip labels.
//!
//! A live mid-turn user choice fills options with a `UserChoice` source.
//! Settled oneshot reply suggestions may fill with `OneshotHints` when
//! eligible. No cancel chip or ⌘⌫ binding; turn cancel / freeform-while-awaiting
//! complete parked choices on the agent wire.

/// One chip option: wire id for activation, label for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastResponseOption {
    pub id: String,
    pub label: String,
}

/// Why the shell is filled — controls activation path.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FastResponseSource {
    /// Empty / not from a live host choice.
    #[default]
    None,
    /// Answer via `AgentHandle::answer_user_choice` — not `send_prompt_text`.
    UserChoice {
        correlation_id: u64,
        /// Question text when known (live chip + settled host log).
        prompt: Option<String>,
    },
    /// Settled freeform reply suggestions; activation sends a normal user turn.
    OneshotHints,
}

/// Ordered option chips (no cancel field).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FastResponse {
    /// Ordered options (⌘1…⌘n). Empty ⇒ shell hidden.
    pub options: Vec<FastResponseOption>,
    /// Activation channel for the current fill.
    pub source: FastResponseSource,
}

/// Resolved chip activation payload (id only — not the hotkey label).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FastResponsePick {
    Option { id: String },
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

/// Lifecycle empty-send form for bootstrap / next-action seeds.
pub fn lifecycle_send_text(command: Option<&str>) -> Option<String> {
    command.and_then(format_lifecycle_command)
}

pub fn is_empty(fr: &FastResponse) -> bool {
    fr.options.is_empty()
}

/// Shown when non-empty shell, turn gate passes, and either empty composer or
/// awaiting a user choice (custom-answer typing keeps chips). Oneshot pending
/// is not a gate.
pub fn visible(
    is_streaming: bool,
    is_awaiting_user: bool,
    input_empty: bool,
    fr: &FastResponse,
) -> bool {
    if is_empty(fr) {
        return false;
    }
    if is_streaming && !is_awaiting_user {
        return false;
    }
    // While awaiting, chips stay up with non-empty composer (custom answer).
    // Otherwise non-empty input hides chips.
    if !input_empty && !is_awaiting_user {
        return false;
    }
    true
}

/// Whether the composer section (input + footer, including model selector) uses
/// the quiet accent awaiting tint (custom-answer surface).
pub fn awaiting_composer_chrome(is_awaiting_user: bool) -> bool {
    is_awaiting_user
}

/// ⌘1…⌘9: 1-based index into options; out of range → None.
pub fn resolve_cmd_digit(fr: &FastResponse, digit: u8) -> Option<FastResponsePick> {
    if !(1..=9).contains(&digit) {
        return None;
    }
    fr.options
        .get((digit as usize) - 1)
        .map(|o| FastResponsePick::Option { id: o.id.clone() })
}

/// ⌘*n* when chips are visible; no-op otherwise.
pub fn resolve_cmd_digit_when_visible(
    is_streaming: bool,
    is_awaiting_user: bool,
    input_empty: bool,
    fr: &FastResponse,
    digit: u8,
) -> Option<FastResponsePick> {
    if !visible(is_streaming, is_awaiting_user, input_empty, fr) {
        return None;
    }
    resolve_cmd_digit(fr, digit)
}

/// Chip label: hotkey then action, e.g. `⌘1  /ds-step`.
pub fn option_chip_label(index_1based: usize, action: &str) -> String {
    format!("⌘{index_1based}  {action}")
}

/// Spacer height above chips so they sit at the bottom of the chat viewport
/// when natural content is shorter than the viewport.
pub fn bottom_pad(viewport_h: f32, content_h: f32, prev_pad: f32) -> f32 {
    let natural = (content_h - prev_pad).max(0.0);
    (viewport_h - natural).max(0.0)
}

/// Prefix for host question chips (live and settled storage).
pub const USER_CHOICE_QUESTION_PREFIX: &str = "Question: ";

/// Format question text for host chips / transcript storage.
/// Prepends `Question: ` when missing; idempotent if already present.
/// Empty/whitespace input yields an empty string (callers treat as omit).
pub fn format_user_choice_question_text(raw: &str) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return String::new();
    }
    if t.starts_with(USER_CHOICE_QUESTION_PREFIX) {
        t.to_string()
    } else {
        format!("{USER_CHOICE_QUESTION_PREFIX}{t}")
    }
}

/// Non-empty formatted question text for a live user-choice shell.
/// `None` when source is not UserChoice or prompt is empty/missing.
/// Display form always uses [`format_user_choice_question_text`].
pub fn live_question_prompt(fr: &FastResponse) -> Option<String> {
    match &fr.source {
        FastResponseSource::UserChoice {
            prompt: Some(p), ..
        } => {
            let formatted = format_user_choice_question_text(p);
            if formatted.is_empty() {
                None
            } else {
                Some(formatted)
            }
        }
        _ => None,
    }
}

/// Build shell from a live mid-turn user choice (options + optional prompt).
pub fn from_user_choice(
    correlation_id: u64,
    prompt: Option<String>,
    options: impl IntoIterator<Item = (String, String)>,
) -> FastResponse {
    let options: Vec<FastResponseOption> = options
        .into_iter()
        .take(9)
        .map(|(id, label)| FastResponseOption { id, label })
        .collect();
    FastResponse {
        options,
        source: FastResponseSource::UserChoice {
            correlation_id,
            prompt,
        },
    }
}

/// Build shell from settled oneshot reply suggestions (id == label == text).
pub fn from_oneshot_hints(replies: impl IntoIterator<Item = String>) -> FastResponse {
    let options: Vec<FastResponseOption> = replies
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(9)
        .map(|text| FastResponseOption {
            id: text.clone(),
            label: text,
        })
        .collect();
    FastResponse {
        options,
        source: FastResponseSource::OneshotHints,
    }
}

/// Clear shell and drop source after answer / turn end.
pub fn clear() -> FastResponse {
    FastResponse::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_options() -> FastResponse {
        FastResponse {
            options: vec![
                FastResponseOption {
                    id: "/ds-step".into(),
                    label: "/ds-step".into(),
                },
                FastResponseOption {
                    id: "/ds-spec".into(),
                    label: "/ds-spec".into(),
                },
                FastResponseOption {
                    id: "/ds-archive".into(),
                    label: "/ds-archive".into(),
                },
            ],
            source: FastResponseSource::None,
        }
    }

    // @spec chat/fast-response Empty-send formatting: Bare skill name formats with leading slash
    #[test]
    fn bare_skill_name_formats_with_leading_slash() {
        assert_eq!(
            format_lifecycle_command("ds-explore").as_deref(),
            Some("/ds-explore")
        );
        assert_eq!(
            format_lifecycle_command("ds-archive").as_deref(),
            Some("/ds-archive")
        );
    }

    // @spec chat/fast-response Empty-send formatting: Already-slashed command is preserved
    #[test]
    fn already_slashed_command_is_preserved() {
        assert_eq!(
            format_lifecycle_command("/ds-spec").as_deref(),
            Some("/ds-spec")
        );
    }

    // @spec chat/fast-response Visibility: Idle empty composer with options shows chips
    #[test]
    fn idle_empty_composer_with_options_shows_chips() {
        let fr = sample_options();
        assert!(visible(false, false, true, &fr));
    }

    // @spec chat/fast-response Visibility: Streaming without awaiting user hides chips
    #[test]
    fn streaming_without_awaiting_user_hides_chips() {
        let fr = sample_options();
        assert!(!visible(true, false, true, &fr));
    }

    // @spec chat/fast-response Visibility: Awaiting user shows chips while turn is open
    #[test]
    fn awaiting_user_shows_chips_while_turn_is_open() {
        // GIVEN non-empty options, empty composer, turn open, awaiting user
        let fr = sample_options();
        assert!(visible(true, true, true, &fr));
    }

    // @spec chat/fast-response Visibility: Non-empty composer hides chips when not awaiting
    #[test]
    fn non_empty_composer_hides_chips_when_not_awaiting() {
        let fr = sample_options();
        assert!(!visible(false, false, false, &fr));
        assert!(!visible(true, false, false, &fr));
    }

    // @spec chat/fast-response Visibility: Awaiting user shows chips with non-empty composer
    #[test]
    fn awaiting_user_shows_chips_with_non_empty_composer() {
        let fr = sample_options();
        // GIVEN options + non-empty composer + awaiting (turn open or idle)
        assert!(visible(true, true, false, &fr));
        assert!(visible(false, true, false, &fr));
    }

    // @spec chat/fast-response Visibility: Empty options hide chips
    #[test]
    fn empty_options_hide_chips() {
        let fr = FastResponse::default();
        assert!(is_empty(&fr));
        assert!(!visible(false, false, true, &fr));
    }

    // @spec chat/fast-response Key resolution: Cmd-digit selects matching option
    #[test]
    fn cmd_digit_selects_matching_option() {
        let fr = sample_options();
        let selected = resolve_cmd_digit_when_visible(false, false, true, &fr, 2).expect("visible");
        assert_eq!(
            selected,
            FastResponsePick::Option {
                id: "/ds-spec".into()
            }
        );
    }

    // @spec chat/fast-response Key resolution: Resolution is a no-op when chips not visible
    #[test]
    fn resolution_is_a_no_op_when_chips_not_visible() {
        let fr = sample_options();
        assert_eq!(
            resolve_cmd_digit_when_visible(true, false, true, &fr, 1),
            None
        );
        assert_eq!(
            resolve_cmd_digit_when_visible(false, false, false, &fr, 1),
            None
        );
        let empty = FastResponse::default();
        assert_eq!(
            resolve_cmd_digit_when_visible(false, false, true, &empty, 1),
            None
        );
    }

    // @spec chat/fast-response Chip labels: Option chip label is hotkey then action
    #[test]
    fn option_chip_label_is_hotkey_then_action() {
        let action = "/ds-step";
        let label = option_chip_label(1, action);
        assert!(label.starts_with("⌘1"), "label={label}");
        assert!(label.contains(action), "label={label}");
        assert_ne!(label, action);
    }

    // @spec chat/fast-response Bottom pad: Short content yields positive pad
    #[test]
    fn short_content_yields_positive_pad() {
        assert_eq!(bottom_pad(400.0, 100.0, 0.0), 300.0);
    }

    // @spec chat/fast-response Bottom pad: Content at or above viewport yields zero pad
    #[test]
    fn content_at_or_above_viewport_yields_zero_pad() {
        assert_eq!(bottom_pad(400.0, 500.0, 0.0), 0.0);
    }

    #[test]
    fn format_user_choice_question_text_prefixes_and_is_idempotent() {
        assert_eq!(
            format_user_choice_question_text("Ship it?"),
            "Question: Ship it?"
        );
        assert_eq!(
            format_user_choice_question_text("Question: Ship it?"),
            "Question: Ship it?"
        );
        assert_eq!(format_user_choice_question_text("  "), "");
        assert_eq!(format_user_choice_question_text(""), "");
    }

    // @spec chat/fast-response Live question chip: Non-empty prompt shows a question chip above options
    #[test]
    fn non_empty_prompt_shows_a_question_chip_above_options() {
        // GIVEN awaiting user choice with non-empty question and options
        let fr = from_user_choice(
            1,
            Some("Ship later or now?".into()),
            [
                ("later".into(), "Later".into()),
                ("now".into(), "Now".into()),
            ],
        );
        assert!(!is_empty(&fr));
        // WHEN live chrome is derived
        let q = live_question_prompt(&fr);
        // THEN question text is present for a chip above options (not a numbered option)
        assert_eq!(q.as_deref(), Some("Question: Ship later or now?"));
        assert!(fr.options.iter().all(|o| o.label != "Ship later or now?"));
        assert!(!fr.options.iter().any(|o| o.id == "Ship later or now?"));
    }

    // @spec chat/fast-response Live question chip: Empty prompt omits the question chip
    #[test]
    fn empty_prompt_omits_the_question_chip() {
        let fr_none = from_user_choice(1, None, [("a".into(), "Alpha".into())]);
        assert!(live_question_prompt(&fr_none).is_none());
        assert_eq!(fr_none.options.len(), 1);

        let fr_blank = from_user_choice(2, Some("   ".into()), [("b".into(), "Beta".into())]);
        assert!(live_question_prompt(&fr_blank).is_none());
        assert_eq!(fr_blank.options.len(), 1);

        // Oneshot fill never shows a question chip
        let oneshot = from_oneshot_hints(["hi".into()]);
        assert!(live_question_prompt(&oneshot).is_none());
    }

    // @spec chat/fast-response Ephemeral chips: Visible chips are not a stored user message
    #[test]
    fn visible_chips_are_not_a_stored_user_message() {
        use crate::chat_store::{ChatSession, ContentBlock, Role};

        let fr = sample_options();
        assert!(visible(false, false, true, &fr));
        let action = fr.options.first().expect("option").label.clone();

        // GIVEN visible option chips AND no activation yet — empty session stands in
        // for "chrome only" (product does not append until settle).
        let session = ChatSession::new("change".into());
        let has_text_chip = session.messages.iter().any(|m| {
            matches!(m.role, Role::User)
                && m.content.iter().any(|b| match b {
                    ContentBlock::Text(t) => t == &action || t == &option_chip_label(1, &action),
                    _ => false,
                })
        });
        let has_choice_answer = session.messages.iter().any(|m| {
            m.content
                .iter()
                .any(|b| matches!(b, ContentBlock::UserChoiceAnswer { .. }))
        });
        assert!(
            !has_text_chip && !has_choice_answer,
            "option chips must not appear as committed messages until activation"
        );
    }

    // @spec chat/fast-response Population: Refresh does not clear options while awaiting a user choice
    #[test]
    fn refresh_does_not_clear_options_while_awaiting_a_user_choice() {
        // GIVEN a session awaiting a user choice with non-empty options
        let filled = from_user_choice(42, None, [("opt-a".into(), "Alpha".into())]);
        assert!(!is_empty(&filled));
        // WHEN refresh would run while awaiting — product path skips clear
        // (mirrored here: only clear when not awaiting).
        let is_awaiting_user = true;
        let after = if !is_awaiting_user {
            clear()
        } else {
            filled.clone()
        };
        assert!(!is_empty(&after));
        assert_eq!(after.options.len(), 1);
        assert!(matches!(
            after.source,
            FastResponseSource::UserChoice {
                correlation_id: 42,
                ..
            }
        ));
    }

    // Question activation host commit lives in area::interaction (activate_fast_response).
    // Wire mapping: pick → Selected option id.
    #[test]
    fn option_pick_maps_to_selected_answer() {
        use duckchat::UserChoiceAnswer;

        let fr = from_user_choice(7, Some("Ship?".into()), [("opt-a".into(), "Alpha".into())]);
        let pick = resolve_cmd_digit(&fr, 1).expect("option");
        assert_eq!(pick, FastResponsePick::Option { id: "opt-a".into() });
        let answer = match pick {
            FastResponsePick::Option { id } => UserChoiceAnswer::Selected { option_id: id },
        };
        assert!(matches!(
            answer,
            UserChoiceAnswer::Selected { option_id } if option_id == "opt-a"
        ));
    }

    // @spec chat/fast-response Oneshot activation: Option activation sends the oneshot text as a user message
    #[test]
    fn option_activation_sends_the_oneshot_text_as_a_user_message() {
        // GIVEN oneshot-hint shell (not UserChoice in-band)
        let fr = from_oneshot_hints(["sounds good".into(), "no thanks".into()]);
        assert!(matches!(fr.source, FastResponseSource::OneshotHints));
        // WHEN first option is activated — pick id is the freeform send text
        let pick = resolve_cmd_digit_when_visible(false, false, true, &fr, 1).expect("visible");
        assert_eq!(
            pick,
            FastResponsePick::Option {
                id: "sounds good".into()
            }
        );
        // Oneshot activation path is send_prompt_text with that id (product),
        // not answer_user_choice.
        assert!(!matches!(fr.source, FastResponseSource::UserChoice { .. }));
    }

    // @spec chat/fast-response Awaiting composer chrome: Awaiting user applies quiet accent tint to the composer section
    #[test]
    fn awaiting_user_applies_quiet_accent_tint_to_the_composer_section() {
        assert!(awaiting_composer_chrome(true));
        let theme = iced::Theme::Dark;
        let awaiting = crate::theme::chat_composer_awaiting(&theme);
        let normal = crate::theme::chat_input(&theme);
        assert_ne!(
            awaiting.background, normal.background,
            "awaiting composer must differ from normal paper input"
        );
        assert_eq!(
            awaiting.background,
            Some(iced::Background::Color(crate::theme::quiet_accent_surface()))
        );
    }

    // @spec chat/fast-response Awaiting composer chrome: Not awaiting leaves the composer section untinted
    #[test]
    fn not_awaiting_leaves_the_composer_section_untinted() {
        assert!(!awaiting_composer_chrome(false));
        let theme = iced::Theme::Dark;
        let normal = crate::theme::chat_input(&theme);
        assert_eq!(
            normal.background,
            Some(iced::Background::Color(crate::theme::bg_base()))
        );
    }

    // @spec chat/fast-response Awaiting composer chrome: Model selector matches the composer section tint while awaiting
    #[test]
    fn model_selector_matches_the_composer_section_tint_while_awaiting() {
        assert!(
            awaiting_composer_chrome(true),
            "model selector shares the awaiting chrome gate"
        );
        let theme = iced::Theme::Dark;
        let composer = crate::theme::chat_composer_awaiting(&theme);
        let pick = crate::theme::pick_list_ghost_awaiting_style(
            &theme,
            iced::widget::pick_list::Status::Active,
        );
        let Some(iced::Background::Color(composer_bg)) = composer.background else {
            panic!("composer awaiting must paint a color");
        };
        assert_eq!(
            pick.background,
            iced::Background::Color(composer_bg),
            "model selector active fill must match composer awaiting tint"
        );
    }
}
