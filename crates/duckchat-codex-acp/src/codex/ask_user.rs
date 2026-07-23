//! Bridge Codex `item/tool/requestUserInput` to ACP parent host choices.
//!
//! Product options (kind `custom`) go through `session/request_permission` so
//! the shared ACP client parks a user-choice chip. Ordinary tool approvals are
//! auto-completed without host UI.

use serde_json::{Value, json};

/// Method name for structured user-input server requests.
pub const REQUEST_USER_INPUT_METHOD: &str = "item/tool/requestUserInput";

/// One option for host choice chips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceOption {
    pub id: String,
    pub label: String,
}

/// One question from `item/tool/requestUserInput` (1–3 per request).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedQuestion {
    pub id: String,
    pub question: Option<String>,
    pub options: Vec<ChoiceOption>,
}

/// Host (or auto) decision for a structured user-input request.
#[derive(Debug, Clone)]
pub enum UserInputDecision {
    /// Accept the questionnaire with answers map (question id → answer strings).
    Accepted { answers: Value },
    /// Cancel without accepting.
    Cancelled,
}

/// Decode `item/tool/requestUserInput` params → all questions (id, text, options).
pub fn decode_user_input(params: &Value) -> Vec<DecodedQuestion> {
    let questions = params
        .get("questions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    questions
        .iter()
        .enumerate()
        .map(|(i, q)| {
            let id = q
                .get("id")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("q{i}"));
            let question = q
                .get("question")
                .and_then(Value::as_str)
                .map(str::to_string);
            let options = q
                .get("options")
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
                            // Wire has no option id; use label as optionId for the parent.
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
            DecodedQuestion {
                id,
                question,
                options,
            }
        })
        .collect()
}

/// Merge single-question answer maps into one ToolRequestUserInput `answers` object.
pub fn merge_answers(parts: impl IntoIterator<Item = Value>) -> Value {
    let mut out = serde_json::Map::new();
    for part in parts {
        if let Some(obj) = part.as_object() {
            for (k, v) in obj {
                out.insert(k.clone(), v.clone());
            }
        }
    }
    Value::Object(out)
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

/// Map host selection → accepted answers for the first question id.
pub fn decision_from_selected(
    question_id: &str,
    option_id: &str,
    options: &[ChoiceOption],
) -> UserInputDecision {
    let label = options
        .iter()
        .find(|o| o.id == option_id)
        .map(|o| o.label.as_str())
        .unwrap_or(option_id);
    UserInputDecision::Accepted {
        answers: answers_map(question_id, label),
    }
}

/// Map host freeform custom text → accepted answers (not skip/cancel).
pub fn decision_from_freeform(question_id: &str, text: &str) -> UserInputDecision {
    UserInputDecision::Accepted {
        answers: answers_map(question_id, text),
    }
}

/// Map host cancel → cancelled (no accepted answers).
pub fn decision_from_cancelled() -> UserInputDecision {
    UserInputDecision::Cancelled
}

fn answers_map(question_id: &str, answer: &str) -> Value {
    // ToolRequestUserInputResponse: answers is map of question id → { answers: [string] }
    json!({
        question_id: {
            "answers": [answer]
        }
    })
}

/// Encode a JSON-RPC response to the App Server for a user-input decision.
pub fn encode_user_input_rpc(id: &Value, decision: &UserInputDecision) -> Value {
    match decision {
        UserInputDecision::Accepted { answers } => json!({
            "id": id,
            "result": { "answers": answers }
        }),
        UserInputDecision::Cancelled => json!({
            "id": id,
            "error": {
                "code": -32000,
                "message": "user cancelled"
            }
        }),
    }
}

/// Method name for elevated network/filesystem permission grants.
pub const PERMISSIONS_REQUEST_APPROVAL_METHOD: &str = "item/permissions/requestApproval";

/// JSON-RPC result body that auto-allows a server approval request.
///
/// - Command / file / legacy exec-patch: `{ "decision": "accept" | "approved" }`
/// - `item/permissions/requestApproval`: grant body echoing requested permissions
///   for the turn (`PermissionsRequestApprovalResponse` shape).
pub fn auto_allow_approval_result(method: &str, params: &Value) -> Value {
    if is_permissions_request_approval_method(method) {
        let permissions = params
            .get("permissions")
            .cloned()
            .unwrap_or_else(|| json!({}));
        return json!({
            "permissions": permissions,
            "scope": "turn",
        });
    }
    if method.contains("execCommandApproval") || method.contains("applyPatchApproval") {
        json!({ "decision": "approved" })
    } else {
        // item/commandExecution/requestApproval, item/fileChange/requestApproval, …
        json!({ "decision": "accept" })
    }
}

/// True when this server request is structured user input (host choice path).
pub fn is_user_input_method(method: &str) -> bool {
    method == REQUEST_USER_INPUT_METHOD || method.ends_with("requestUserInput")
}

/// True when this is the elevated permissions grant server request.
pub fn is_permissions_request_approval_method(method: &str) -> bool {
    method == PERMISSIONS_REQUEST_APPROVAL_METHOD || method.ends_with("permissions/requestApproval")
}

/// True when this server request is an auto-allowable approval (command, file,
/// permissions grant, or legacy exec/patch) — not host UI.
pub fn is_ordinary_approval_method(method: &str) -> bool {
    if is_user_input_method(method) {
        return false;
    }
    is_permissions_request_approval_method(method)
        || method.contains("Approval")
        || method.contains("approval")
        || method == "execCommandApproval"
        || method == "applyPatchApproval"
}

/// Parse parent `session/request_permission` result into a decision.
///
/// Parent encodes selected/cancelled/custom via outcome shapes the shared
/// client already uses for product options.
pub fn decision_from_parent_result(
    result: &Value,
    question_id: &str,
    options: &[ChoiceOption],
) -> UserInputDecision {
    let outcome = result
        .pointer("/outcome/outcome")
        .or_else(|| result.pointer("/outcome"))
        .and_then(Value::as_str)
        .unwrap_or("");
    match outcome {
        "cancelled" | "cancel" => decision_from_cancelled(),
        "selected" => {
            let option_id = result
                .pointer("/outcome/optionId")
                .or_else(|| result.pointer("/outcome/option_id"))
                .and_then(Value::as_str)
                .unwrap_or("");
            // Freeform: optionId may be custom text not in the listed options.
            if options.iter().any(|o| o.id == option_id) {
                decision_from_selected(question_id, option_id, options)
            } else if !option_id.is_empty() {
                decision_from_freeform(question_id, option_id)
            } else {
                decision_from_cancelled()
            }
        }
        _ => {
            // Some clients put optionId at top level.
            if let Some(option_id) = result.get("optionId").and_then(Value::as_str) {
                if options.iter().any(|o| o.id == option_id) {
                    return decision_from_selected(question_id, option_id, options);
                }
                return decision_from_freeform(question_id, option_id);
            }
            decision_from_cancelled()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @spec harness/openai-codex Mid-turn structured questions: A structured user-input request surfaces a host user choice
    #[test]
    fn structured_user_input_surfaces_host_user_choice() {
        let params = json!({
            "itemId": "item-1",
            "threadId": "t1",
            "turnId": "turn-1",
            "questions": [{
                "id": "q1",
                "header": "Ship",
                "question": "Ship it?",
                "options": [
                    { "label": "Yes", "description": "ship" },
                    { "label": "No", "description": "hold" }
                ]
            }]
        });
        let qs = decode_user_input(&params);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].question.as_deref(), Some("Ship it?"));
        assert_eq!(qs[0].id, "q1");
        let acp = permission_request_params(
            "sess-1",
            "item-1",
            qs[0].question.as_deref(),
            &qs[0].options,
        );
        let kinds: Vec<&str> = acp["options"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["kind"].as_str().unwrap())
            .collect();
        assert!(kinds.iter().all(|k| *k == "custom"));
        assert_eq!(acp["options"][0]["name"], "Yes");
        assert_eq!(acp["toolCall"]["title"], "Ship it?");
    }

    #[test]
    fn multi_question_decode_and_merge_answers() {
        let params = json!({
            "questions": [
                {
                    "id": "q1",
                    "header": "A",
                    "question": "First?",
                    "options": [{ "label": "One", "description": "" }]
                },
                {
                    "id": "q2",
                    "header": "B",
                    "question": "Second?",
                    "options": [
                        { "label": "Two", "description": "" },
                        { "label": "Three", "description": "" }
                    ]
                }
            ]
        });
        let qs = decode_user_input(&params);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].id, "q1");
        assert_eq!(qs[1].id, "q2");
        assert_eq!(qs[1].options.len(), 2);

        let d1 = decision_from_selected("q1", "One", &qs[0].options);
        let d2 = decision_from_selected("q2", "Two", &qs[1].options);
        let (a1, a2) = match (d1, d2) {
            (
                UserInputDecision::Accepted { answers: a1 },
                UserInputDecision::Accepted { answers: a2 },
            ) => (a1, a2),
            _ => panic!("expected both accepted"),
        };
        let merged = merge_answers([a1, a2]);
        assert_eq!(merged["q1"]["answers"][0], "One");
        assert_eq!(merged["q2"]["answers"][0], "Two");
        let rpc =
            encode_user_input_rpc(&json!(1), &UserInputDecision::Accepted { answers: merged });
        assert_eq!(rpc["result"]["answers"]["q1"]["answers"][0], "One");
        assert_eq!(rpc["result"]["answers"]["q2"]["answers"][0], "Two");
    }

    #[test]
    fn multi_question_cancel_aborts_whole_questionnaire() {
        // Any cancel yields a cancelled RPC (no partial accepted answers).
        let decision = decision_from_cancelled();
        let rpc = encode_user_input_rpc(&json!(2), &decision);
        assert!(rpc.get("error").is_some());
        assert!(rpc.get("result").is_none());
        // Merge is not applied when cancelled — only Accepted carries answers.
        assert!(matches!(decision, UserInputDecision::Cancelled));
    }

    /// @spec harness/openai-codex Mid-turn structured questions: Host selection completes with accepted answers
    #[test]
    fn host_selection_completes_with_accepted_answers() {
        let opts = vec![
            ChoiceOption {
                id: "Yes".into(),
                label: "Yes".into(),
            },
            ChoiceOption {
                id: "No".into(),
                label: "No".into(),
            },
        ];
        let decision = decision_from_selected("q1", "Yes", &opts);
        let rpc = encode_user_input_rpc(&json!(42), &decision);
        assert!(rpc.get("result").is_some());
        assert_eq!(rpc["result"]["answers"]["q1"]["answers"][0], "Yes");
        assert!(rpc.get("error").is_none());
    }

    /// @spec harness/openai-codex Mid-turn structured questions: Host custom freeform completes with accepted free-text answers
    #[test]
    fn host_custom_freeform_completes_with_accepted_free_text() {
        let free = "something else";
        let decision = decision_from_freeform("q1", free);
        let rpc = encode_user_input_rpc(&json!(7), &decision);
        assert!(rpc.get("result").is_some(), "must accept, not cancel");
        assert_eq!(rpc["result"]["answers"]["q1"]["answers"][0], free);
        assert!(rpc.get("error").is_none());
    }

    /// @spec harness/openai-codex Mid-turn structured questions: Host cancel completes without accepting the questionnaire
    #[test]
    fn host_cancel_completes_without_accepting() {
        let decision = decision_from_cancelled();
        let rpc = encode_user_input_rpc(&json!(9), &decision);
        assert!(rpc.get("error").is_some());
        assert!(
            rpc.get("result").is_none(),
            "cancel must not accept with answers"
        );
    }

    /// @spec harness/openai-codex Ordinary tools stay auto-approved: Ordinary tool permission does not require host UI
    #[test]
    fn ordinary_tool_permission_auto_allows_without_host_ui() {
        assert!(is_ordinary_approval_method(
            "item/commandExecution/requestApproval"
        ));
        assert!(!is_user_input_method(
            "item/commandExecution/requestApproval"
        ));
        let empty = json!({});
        let result = auto_allow_approval_result("item/commandExecution/requestApproval", &empty);
        assert_eq!(result["decision"], "accept");
        assert!(result.get("permissions").is_none());

        let file = auto_allow_approval_result("item/fileChange/requestApproval", &empty);
        assert_eq!(file["decision"], "accept");

        let exec = auto_allow_approval_result("execCommandApproval", &empty);
        assert_eq!(exec["decision"], "approved");

        // User-input is the structured-choice path, not auto-allow.
        assert!(is_user_input_method(REQUEST_USER_INPUT_METHOD));
        assert!(!is_ordinary_approval_method(REQUEST_USER_INPUT_METHOD));
    }

    #[test]
    fn permissions_request_approval_auto_grants_requested_profile() {
        assert!(is_permissions_request_approval_method(
            PERMISSIONS_REQUEST_APPROVAL_METHOD
        ));
        assert!(is_ordinary_approval_method(
            PERMISSIONS_REQUEST_APPROVAL_METHOD
        ));
        assert!(!is_user_input_method(PERMISSIONS_REQUEST_APPROVAL_METHOD));

        let params = json!({
            "permissions": {
                "network": { "enabled": true },
                "fileSystem": {
                    "entries": [{
                        "access": "write",
                        "path": { "type": "special", "value": { "kind": "project_roots" } }
                    }]
                }
            }
        });
        let result = auto_allow_approval_result(PERMISSIONS_REQUEST_APPROVAL_METHOD, &params);
        // PermissionsRequestApprovalResponse: permissions + scope, not decision.
        assert!(result.get("decision").is_none());
        assert_eq!(result["scope"], "turn");
        assert_eq!(result["permissions"], params["permissions"]);

        let missing = auto_allow_approval_result(PERMISSIONS_REQUEST_APPROVAL_METHOD, &json!({}));
        assert_eq!(missing["permissions"], json!({}));
        assert_eq!(missing["scope"], "turn");
    }

    #[test]
    fn parent_result_selected_and_freeform_and_cancel() {
        let opts = vec![ChoiceOption {
            id: "A".into(),
            label: "A".into(),
        }];
        let selected = json!({
            "outcome": { "outcome": "selected", "optionId": "A" }
        });
        match decision_from_parent_result(&selected, "q1", &opts) {
            UserInputDecision::Accepted { answers } => {
                assert_eq!(answers["q1"]["answers"][0], "A");
            }
            other => panic!("expected accepted, got {other:?}"),
        }

        let free = json!({
            "outcome": { "outcome": "selected", "optionId": "typed free" }
        });
        match decision_from_parent_result(&free, "q1", &opts) {
            UserInputDecision::Accepted { answers } => {
                assert_eq!(answers["q1"]["answers"][0], "typed free");
            }
            other => panic!("expected freeform accepted, got {other:?}"),
        }

        let cancel = json!({
            "outcome": { "outcome": "cancelled" }
        });
        assert!(matches!(
            decision_from_parent_result(&cancel, "q1", &opts),
            UserInputDecision::Cancelled
        ));
    }
}
