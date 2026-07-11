//! Grok `x.ai/ask_user_question` decode/encode helpers.
//!
//! Neutral options for the host chips, and live-proven
//! `outcome`-tagged results for the agent wire.

use serde_json::{Value, json};

use crate::event::UserChoiceOption;

/// Live Grok ACP method name (leading underscore). Unprefixed alias also accepted.
pub const ASK_USER_METHOD: &str = "_x.ai/ask_user_question";
pub const ASK_USER_METHOD_ALIAS: &str = "x.ai/ask_user_question";

/// True when `method` is a Grok ask-user extension (live or alias form).
pub fn is_ask_user_method(method: &str) -> bool {
    method == ASK_USER_METHOD || method == ASK_USER_METHOD_ALIAS
}

/// Decode questionnaire params into (first question text, option chips).
/// v1: sequential single-select — only the first question is exposed.
pub fn decode_options(params: &Value) -> (Option<String>, Vec<UserChoiceOption>) {
    let questions = params
        .get("questions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let first = questions.first();
    let prompt = first
        .and_then(|q| q.get("question"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let options = first
        .and_then(|q| q.get("options"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|o| {
                    let label = o
                        .get("label")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    if label.is_empty() {
                        return None;
                    }
                    // Wire id defaults to label (v1 sequential single-select).
                    let id = o
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or(&label)
                        .to_string();
                    Some(UserChoiceOption { id, label })
                })
                .collect()
        })
        .unwrap_or_default();
    (prompt, options)
}

/// Encode host selection as live-proven accepted outcome:
/// `{ "outcome": "accepted", "answers": {…}, "partial_answers": null }`.
pub fn encode_selected(question_text: &str, option_label: &str) -> Value {
    json!({
        "outcome": "accepted",
        "answers": { question_text: option_label },
        "partial_answers": null,
    })
}

/// Encode host cancel as live-proven skip:
/// `{ "outcome": "skip_interview" }`.
pub fn encode_cancelled() -> Value {
    json!({ "outcome": "skip_interview" })
}

/// Resolve the option label for a host selection (id may already be the label).
pub fn label_for_selection(options: &[UserChoiceOption], option_id: &str) -> String {
    options
        .iter()
        .find(|o| o.id == option_id)
        .map(|o| o.label.clone())
        .unwrap_or_else(|| option_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_matches_live_outcome_shapes() {
        let accepted = encode_selected("Ship?", "Yes");
        assert_eq!(
            accepted,
            json!({
                "outcome": "accepted",
                "answers": { "Ship?": "Yes" },
                "partial_answers": null,
            })
        );
        let skip = encode_cancelled();
        assert_eq!(skip, json!({ "outcome": "skip_interview" }));
    }

    // @spec harness/grok Question wire mapping: Host custom freeform answer completes with an accepted free-text answer
    #[test]
    fn host_custom_freeform_answer_completes_with_an_accepted_free_text_answer() {
        let free = "something else";
        let result = encode_selected("Ship?", free);
        assert_eq!(result["outcome"], "accepted", "result={result}");
        assert_eq!(result["answers"]["Ship?"], free);
        assert!(result["partial_answers"].is_null());
        assert_ne!(result["outcome"], "skip_interview");
    }

    #[test]
    fn method_names_match_live_capture() {
        assert_eq!(ASK_USER_METHOD, "_x.ai/ask_user_question");
        assert!(is_ask_user_method(ASK_USER_METHOD));
        assert!(is_ask_user_method(ASK_USER_METHOD_ALIAS));
    }
}
