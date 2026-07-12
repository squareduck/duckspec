//! Codex App Server notifications → ACP dialect-profile `session/update` params.
//!
//! Shapes match what `duckchat::acp::map_update` already accepts from Grok/Claude
//! (`agent_message_chunk`, `agent_thought_chunk`, `tool_call` /
//! `tool_call_update`, `_meta.totalTokens`).

use serde_json::{Value, json};

/// Map one App Server notification (full `{ method, params }` object) into zero
/// or more profile `session/update` **params** objects.
pub fn map_notification(notif: &Value, session_id: &str) -> Vec<Value> {
    let method = notif.get("method").and_then(Value::as_str).unwrap_or("");
    let params = notif.get("params").unwrap_or(&Value::Null);

    match method {
        "item/agentMessage/delta" => map_agent_message_delta(params, session_id),
        "item/reasoning/textDelta" | "item/reasoning/summaryTextDelta" => {
            map_reasoning_delta(params, session_id)
        }
        "item/started" => map_item_started(params, session_id),
        "item/completed" => map_item_completed(params, session_id),
        "thread/tokenUsage/updated" => map_token_usage(params, session_id),
        _ => Vec::new(),
    }
}

fn map_agent_message_delta(params: &Value, session_id: &str) -> Vec<Value> {
    let Some(delta) = params.get("delta").and_then(Value::as_str) else {
        return Vec::new();
    };
    if delta.is_empty() {
        return Vec::new();
    }
    vec![agent_message_chunk(session_id, delta)]
}

fn map_reasoning_delta(params: &Value, session_id: &str) -> Vec<Value> {
    let Some(delta) = params.get("delta").and_then(Value::as_str) else {
        return Vec::new();
    };
    if delta.is_empty() {
        return Vec::new();
    }
    vec![agent_thought_chunk(session_id, delta)]
}

fn map_item_started(params: &Value, session_id: &str) -> Vec<Value> {
    let Some(item) = params.get("item") else {
        return Vec::new();
    };
    match item.get("type").and_then(Value::as_str) {
        Some("commandExecution") | Some("mcpToolCall") | Some("webSearch") | Some("fileChange") => {
            tool_call_from_item(item, session_id)
                .into_iter()
                .collect()
        }
        // agentMessage streams via item/agentMessage/delta; no started map.
        _ => Vec::new(),
    }
}

fn map_item_completed(params: &Value, session_id: &str) -> Vec<Value> {
    let Some(item) = params.get("item") else {
        return Vec::new();
    };
    match item.get("type").and_then(Value::as_str) {
        Some("commandExecution") | Some("mcpToolCall") | Some("webSearch") | Some("fileChange") => {
            tool_result_from_item(item, session_id)
                .into_iter()
                .collect()
        }
        // agentMessage / reasoning stream via delta notifications; completed
        // carries the full aggregate and would double-emit if mapped here.
        _ => Vec::new(),
    }
}

fn map_token_usage(params: &Value, session_id: &str) -> Vec<Value> {
    // Prefer cumulative total.totalTokens; fall back to last.totalTokens.
    let total = params
        .pointer("/tokenUsage/total/totalTokens")
        .and_then(Value::as_u64)
        .or_else(|| {
            params
                .pointer("/tokenUsage/last/totalTokens")
                .and_then(Value::as_u64)
        });
    let Some(total) = total else {
        return Vec::new();
    };
    vec![json!({
        "sessionId": session_id,
        "_meta": { "totalTokens": total }
    })]
}

fn tool_call_from_item(item: &Value, session_id: &str) -> Option<Value> {
    let id = item.get("id").and_then(Value::as_str)?;
    let (title, raw_input) = tool_title_and_input(item);
    Some(json!({
        "sessionId": session_id,
        "update": {
            "sessionUpdate": "tool_call",
            "toolCallId": id,
            "title": title,
            "status": "pending",
            "rawInput": raw_input,
        }
    }))
}

fn tool_result_from_item(item: &Value, session_id: &str) -> Option<Value> {
    let id = item.get("id").and_then(Value::as_str)?;
    let (title, _) = tool_title_and_input(item);
    let output = tool_output_text(item);
    Some(json!({
        "sessionId": session_id,
        "update": {
            "sessionUpdate": "tool_call_update",
            "toolCallId": id,
            "title": title,
            "status": "completed",
            "content": [{
                "type": "content",
                "content": { "type": "text", "text": output }
            }]
        }
    }))
}

fn tool_title_and_input(item: &Value) -> (String, Value) {
    match item.get("type").and_then(Value::as_str) {
        Some("commandExecution") => {
            let cmd = item
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("commandExecution");
            (
                cmd.to_string(),
                json!({
                    "command": item.get("command").cloned().unwrap_or(Value::Null),
                    "cwd": item.get("cwd").cloned().unwrap_or(Value::Null),
                }),
            )
        }
        Some("mcpToolCall") => {
            let tool = item
                .get("tool")
                .and_then(Value::as_str)
                .unwrap_or("mcpToolCall");
            let server = item.get("server").and_then(Value::as_str).unwrap_or("");
            let title = if server.is_empty() {
                tool.to_string()
            } else {
                format!("{server}/{tool}")
            };
            (
                title,
                item.get("arguments").cloned().unwrap_or(json!({})),
            )
        }
        Some("webSearch") => (
            "webSearch".into(),
            json!({ "query": item.get("query").cloned().unwrap_or(Value::Null) }),
        ),
        Some("fileChange") => (
            "fileChange".into(),
            json!({ "changes": item.get("changes").cloned().unwrap_or(json!([])) }),
        ),
        _ => (
            item.get("type")
                .and_then(Value::as_str)
                .unwrap_or("tool")
                .to_string(),
            json!({}),
        ),
    }
}

fn tool_output_text(item: &Value) -> String {
    match item.get("type").and_then(Value::as_str) {
        Some("commandExecution") => item
            .get("aggregatedOutput")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        Some("mcpToolCall") => {
            if let Some(err) = item.get("error").filter(|e| !e.is_null()) {
                return err.to_string();
            }
            item.get("result")
                .map(|r| {
                    r.as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| r.to_string())
                })
                .unwrap_or_default()
        }
        Some("webSearch") => item
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        Some("fileChange") => item
            .get("status")
            .map(|s| {
                s.as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| s.to_string())
            })
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn agent_message_chunk(session_id: &str, text: &str) -> Value {
    json!({
        "sessionId": session_id,
        "update": {
            "sessionUpdate": "agent_message_chunk",
            "content": {
                "type": "text",
                "text": text,
            }
        }
    })
}

fn agent_thought_chunk(session_id: &str, text: &str) -> Value {
    json!({
        "sessionId": session_id,
        "update": {
            "sessionUpdate": "agent_thought_chunk",
            "content": {
                "type": "text",
                "text": text,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @spec harness/openai-codex Profile-compatible event emission: Assistant text surfaces as profile content updates
    #[test]
    fn assistant_text_maps_to_agent_message_chunk() {
        let notif = json!({
            "method": "item/agentMessage/delta",
            "params": {
                "threadId": "t1",
                "turnId": "turn-1",
                "itemId": "msg-1",
                "delta": "hello from codex"
            }
        });
        let updates = map_notification(&notif, "sess-1");
        assert_eq!(updates.len(), 1);
        assert_eq!(
            updates[0]["update"]["sessionUpdate"],
            "agent_message_chunk"
        );
        assert_eq!(updates[0]["update"]["content"]["text"], "hello from codex");
        assert_eq!(updates[0]["sessionId"], "sess-1");
    }

    /// @spec harness/openai-codex Profile-compatible event emission: A tool call surfaces as profile tool use then completed result
    #[test]
    fn tool_call_maps_to_profile_use_then_result() {
        let started = json!({
            "method": "item/started",
            "params": {
                "threadId": "t1",
                "turnId": "turn-1",
                "item": {
                    "type": "commandExecution",
                    "id": "call-9",
                    "command": "ls -la",
                    "cwd": "/tmp",
                    "status": "inProgress",
                    "commandActions": []
                }
            }
        });
        let completed = json!({
            "method": "item/completed",
            "params": {
                "threadId": "t1",
                "turnId": "turn-1",
                "item": {
                    "type": "commandExecution",
                    "id": "call-9",
                    "command": "ls -la",
                    "cwd": "/tmp",
                    "status": "completed",
                    "commandActions": [],
                    "aggregatedOutput": "file.rs\n"
                }
            }
        });

        let use_u = map_notification(&started, "s");
        assert_eq!(use_u.len(), 1);
        assert_eq!(use_u[0]["update"]["sessionUpdate"], "tool_call");
        assert_eq!(use_u[0]["update"]["toolCallId"], "call-9");
        assert_eq!(use_u[0]["update"]["title"], "ls -la");
        assert_eq!(use_u[0]["update"]["rawInput"]["command"], "ls -la");

        let res_u = map_notification(&completed, "s");
        assert_eq!(res_u.len(), 1);
        assert_eq!(res_u[0]["update"]["sessionUpdate"], "tool_call_update");
        assert_eq!(res_u[0]["update"]["status"], "completed");
        assert_eq!(res_u[0]["update"]["toolCallId"], "call-9");
        assert_eq!(
            res_u[0]["update"]["content"][0]["content"]["text"],
            "file.rs\n"
        );
    }

    /// @spec harness/openai-codex Profile-compatible event emission: Token telemetry surfaces as usage with total tokens
    #[test]
    fn token_telemetry_maps_to_total_tokens() {
        let notif = json!({
            "method": "thread/tokenUsage/updated",
            "params": {
                "threadId": "t1",
                "turnId": "turn-1",
                "tokenUsage": {
                    "last": {
                        "inputTokens": 10,
                        "cachedInputTokens": 0,
                        "outputTokens": 5,
                        "reasoningOutputTokens": 0,
                        "totalTokens": 15
                    },
                    "total": {
                        "inputTokens": 100,
                        "cachedInputTokens": 20,
                        "outputTokens": 40,
                        "reasoningOutputTokens": 0,
                        "totalTokens": 160
                    }
                }
            }
        });
        let updates = map_notification(&notif, "sess-1");
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0]["_meta"]["totalTokens"], 160);
        assert_eq!(updates[0]["sessionId"], "sess-1");
    }

    #[test]
    fn mcp_tool_call_uses_server_tool_title() {
        let started = json!({
            "method": "item/started",
            "params": {
                "item": {
                    "type": "mcpToolCall",
                    "id": "mcp-1",
                    "server": "docs",
                    "tool": "search",
                    "arguments": { "q": "x" },
                    "status": "inProgress"
                }
            }
        });
        let u = map_notification(&started, "s");
        assert_eq!(u[0]["update"]["title"], "docs/search");
        assert_eq!(u[0]["update"]["rawInput"]["q"], "x");
    }
}
