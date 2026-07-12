//! Bridge Claude `AskUserQuestion` control requests to ACP parent choices.

use serde_json::{Value, json};

/// Tool name Claude uses for structured clarifying questions.
pub const ASK_USER_QUESTION: &str = "AskUserQuestion";

/// Host (or auto) decision for a Claude permission / canUseTool control request.
#[derive(Debug, Clone)]
pub enum PermissionDecision {
    Allow {
        /// When set, Claude receives rewritten tool input (answers map).
        updated_input: Option<Value>,
    },
    Deny {
        message: String,
    },
}

/// One option extracted from AskUserQuestion input for the parent chips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceOption {
    pub id: String,
    pub label: String,
}

/// Parse AskUserQuestion tool input → (question text, options). v1: first question only.
pub fn decode_ask_user_input(input: &Value) -> (Option<String>, Vec<ChoiceOption>) {
    let questions = input
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
                    let id = o
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or(&label)
                        .to_string();
                    Some(ChoiceOption { id, label })
                })
                .collect()
        })
        .unwrap_or_default();
    (prompt, options)
}

/// Build ACP `session/request_permission` params for product (non allow/reject) options.
pub fn permission_request_params(
    session_id: &str,
    tool_call_id: &str,
    question: Option<&str>,
    options: &[ChoiceOption],
) -> Value {
    let title = question.unwrap_or("Question");
    let acp_options: Vec<Value> = options
        .iter()
        .map(|o| {
            json!({
                "optionId": o.id,
                "name": o.label,
                // Not an allow/reject kind — parent treats this as a user choice.
                "kind": "custom",
            })
        })
        .collect();
    json!({
        "sessionId": session_id,
        "toolCall": {
            "toolCallId": tool_call_id,
            "title": title,
        },
        "options": acp_options,
    })
}

/// Map host selection → allow + answers (question text → option label).
pub fn decision_from_selected(
    questions_input: &Value,
    question_text: &str,
    option_id: &str,
    options: &[ChoiceOption],
) -> PermissionDecision {
    let label = options
        .iter()
        .find(|o| o.id == option_id)
        .map(|o| o.label.as_str())
        .unwrap_or(option_id);
    let mut answers = serde_json::Map::new();
    answers.insert(question_text.to_string(), json!(label));
    let updated = json!({
        "questions": questions_input.get("questions").cloned().unwrap_or(json!([])),
        "answers": answers,
    });
    PermissionDecision::Allow {
        updated_input: Some(updated),
    }
}

/// Map host cancel → deny without accepting the questionnaire.
pub fn decision_from_cancelled() -> PermissionDecision {
    PermissionDecision::Deny {
        message: "User cancelled the question".into(),
    }
}

/// Ordinary tools under bypass: auto-allow without host UI.
pub fn auto_allow_ordinary_tool() -> PermissionDecision {
    PermissionDecision::Allow {
        updated_input: None,
    }
}

/// Encode a Claude stream-json control_response for a permission decision.
pub fn encode_control_response(request_id: &str, decision: &PermissionDecision) -> Value {
    let response_body = match decision {
        PermissionDecision::Allow {
            updated_input: Some(input),
        } => json!({
            "behavior": "allow",
            "updatedInput": input,
        }),
        PermissionDecision::Allow {
            updated_input: None,
        } => json!({
            "behavior": "allow",
        }),
        PermissionDecision::Deny { message } => json!({
            "behavior": "deny",
            "message": message,
        }),
    };
    json!({
        "type": "control_response",
        "response": {
            "subtype": "success",
            "request_id": request_id,
            "response": response_body,
        }
    })
}

/// Extract a permission/can_use_tool control request from a raw Claude line.
/// Returns `(request_id, tool_name, tool_input)`.
pub fn parse_control_permission(raw: &Value) -> Option<(String, String, Value)> {
    let type_ = raw.get("type").and_then(Value::as_str)?;
    let request = match type_ {
        "sdk_control_request" | "control_request" => raw.get("request")?,
        "control" => {
            // Some builds nest under request; others put subtype on the root.
            raw.get("request").unwrap_or(raw)
        }
        _ => return None,
    };
    let subtype = request
        .get("subtype")
        .and_then(Value::as_str)
        .or_else(|| raw.get("subtype").and_then(Value::as_str))?;
    if subtype != "permission" && subtype != "can_use_tool" {
        return None;
    }
    // Live Claude 2.1 puts request_id at the top level of control_request;
    // older/sdk shapes nest it under request. Prefer top-level, fall back nested.
    let request_id = raw
        .get("request_id")
        .or_else(|| raw.get("requestId"))
        .or_else(|| request.get("request_id"))
        .or_else(|| request.get("requestId"))
        .and_then(Value::as_str)?
        .to_string();
    let tool_name = request
        .get("tool_name")
        .or_else(|| request.get("toolName"))
        .and_then(Value::as_str)?
        .to_string();
    let tool_input = request
        .get("tool_input")
        .or_else(|| request.get("input"))
        .or_else(|| request.get("toolInput"))
        .cloned()
        .unwrap_or(json!({}));
    Some((request_id, tool_name, tool_input))
}

#[cfg(test)]
mod tests {
    use super::*;

    // @spec harness/claude Mid-prompt parent choice: Host selection completes with allow and answers
    #[test]
    fn host_selection_completes_with_allow_and_answers() {
        let input = json!({
            "questions": [{
                "question": "Ship it?",
                "options": [
                    { "label": "Yes" },
                    { "label": "No" }
                ]
            }]
        });
        let (q, opts) = decode_ask_user_input(&input);
        assert_eq!(q.as_deref(), Some("Ship it?"));
        let decision = decision_from_selected(&input, "Ship it?", "Yes", &opts);
        let wire = encode_control_response("perm_1", &decision);
        assert_eq!(wire["response"]["response"]["behavior"], "allow");
        assert_eq!(
            wire["response"]["response"]["updatedInput"]["answers"]["Ship it?"],
            "Yes"
        );
        assert!(
            wire["response"]["response"]["updatedInput"]["questions"]
                .as_array()
                .is_some_and(|a| !a.is_empty())
        );
    }

    // @spec harness/claude Mid-prompt parent choice: Host custom freeform answer completes with allow and free-text answers
    #[test]
    fn host_custom_freeform_answer_completes_with_allow_and_free_text_answers() {
        let input = json!({
            "questions": [{
                "question": "Ship it?",
                "options": [
                    { "label": "Yes" },
                    { "label": "No" }
                ]
            }]
        });
        let (q, opts) = decode_ask_user_input(&input);
        let free = "something else";
        // Custom freeform: option_id is free text (not a listed option label).
        let decision = decision_from_selected(&input, q.as_deref().unwrap_or(""), free, &opts);
        let wire = encode_control_response("perm_custom", &decision);
        assert_eq!(wire["response"]["response"]["behavior"], "allow");
        assert_eq!(
            wire["response"]["response"]["updatedInput"]["answers"]["Ship it?"],
            free
        );
        assert_ne!(wire["response"]["response"]["behavior"], "deny");
    }

    // @spec harness/claude Mid-prompt parent choice: Host cancel completes without accepting the questionnaire
    #[test]
    fn host_cancel_completes_without_accepting_the_questionnaire() {
        let decision = decision_from_cancelled();
        let wire = encode_control_response("perm_2", &decision);
        assert_eq!(wire["response"]["response"]["behavior"], "deny");
        assert!(
            wire["response"]["response"]
                .get("updatedInput")
                .is_none_or(|v| v.is_null()),
            "cancel must not accept with answers"
        );
    }

    // @spec harness/claude Ordinary tools stay auto-approved: Non-question tools do not require host UI under bypass
    #[test]
    fn non_question_tools_do_not_require_host_ui_under_bypass() {
        // Ordinary tool control → auto-allow; no permission_request_params used.
        let decision = auto_allow_ordinary_tool();
        match decision {
            PermissionDecision::Allow { updated_input: None } => {}
            other => panic!("expected bare allow, got {other:?}"),
        }
        let wire = encode_control_response("perm_bash", &decision);
        assert_eq!(wire["response"]["response"]["behavior"], "allow");
        assert!(wire["response"]["response"].get("updatedInput").is_none());
    }

    // @spec harness/claude Mid-prompt parent choice: An AskUserQuestion request surfaces a host user choice
    #[test]
    fn an_ask_user_question_request_surfaces_a_host_user_choice() {
        // Live Claude 2.1 wire: top-level request_id, can_use_tool, nested input.
        let raw = json!({
            "type": "control_request",
            "request_id": "uuid-live-1",
            "request": {
                "subtype": "can_use_tool",
                "tool_name": "AskUserQuestion",
                "input": {
                    "questions": [{
                        "question": "Pick one",
                        "options": [{ "label": "A" }, { "label": "B" }]
                    }]
                }
            }
        });
        let (id, name, input) = parse_control_permission(&raw).expect("parse live wire");
        assert_eq!(id, "uuid-live-1");
        assert_eq!(name, ASK_USER_QUESTION);
        let (q, opts) = decode_ask_user_input(&input);
        let params = permission_request_params("sess-1", "call-q", q.as_deref(), &opts);
        // Parent AcpTurn classifies non allow/reject kinds as user choice.
        let kinds: Vec<&str> = params["options"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["kind"].as_str().unwrap())
            .collect();
        assert!(kinds.iter().all(|k| *k == "custom"));
        assert_eq!(params["options"][0]["name"], "A");
        assert_eq!(params["toolCall"]["title"], "Pick one");
    }

    #[test]
    fn parse_sdk_control_permission_request() {
        let raw = json!({
            "type": "sdk_control_request",
            "request": {
                "subtype": "permission",
                "request_id": "perm_9",
                "tool_name": "AskUserQuestion",
                "tool_input": { "questions": [] }
            }
        });
        let (id, name, _) = parse_control_permission(&raw).expect("parse");
        assert_eq!(id, "perm_9");
        assert_eq!(name, ASK_USER_QUESTION);
    }

    /// Live Claude 2.1: top-level request_id + can_use_tool + input questions.
    #[test]
    fn parse_live_control_request_top_level_request_id() {
        let raw = json!({
            "type": "control_request",
            "request_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
            "request": {
                "subtype": "can_use_tool",
                "tool_name": "AskUserQuestion",
                "input": {
                    "questions": [{
                        "question": "Ship it?",
                        "options": [
                            { "label": "Yes" },
                            { "label": "No" }
                        ]
                    }]
                }
            }
        });
        let (id, name, input) = parse_control_permission(&raw).expect("parse live shape");
        assert_eq!(id, "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
        assert_eq!(name, ASK_USER_QUESTION);
        let (q, opts) = decode_ask_user_input(&input);
        assert_eq!(q.as_deref(), Some("Ship it?"));
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[0].label, "Yes");
    }
}
