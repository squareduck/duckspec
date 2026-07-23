//! Claude stream-json lines → ACP dialect-profile `session/update` params.
//!
//! Shapes match what `duckchat::acp::map_update` already accepts from Grok
//! (`agent_message_chunk`, `agent_thought_chunk`, `tool_call` /
//! `tool_call_update`, `_meta.totalTokens`).

use serde_json::{Value, json};

use super::protocol::ProtocolMsg;

/// Map one Claude protocol message into zero or more profile `session/update`
/// **params** objects (not full JSON-RPC envelopes).
pub fn claude_line_to_updates(msg: &ProtocolMsg, session_id: &str) -> Vec<Value> {
    let mut updates = Vec::new();

    match msg.type_.as_str() {
        "stream_event" => {
            if let Some(event) = &msg.event
                && event.type_ == "content_block_delta"
                && let Some(delta) = &event.delta
            {
                match delta.type_.as_deref() {
                    Some("thinking_delta") => {
                        if let Some(thinking) = &delta.thinking
                            && !thinking.is_empty()
                        {
                            updates.push(agent_thought_chunk(session_id, thinking));
                        }
                    }
                    Some("text_delta") | None => {
                        if let Some(text) = &delta.text
                            && !text.is_empty()
                        {
                            updates.push(agent_message_chunk(session_id, text));
                        }
                    }
                    // signature_delta and other partials are not profile content.
                    _ => {}
                }
            }
        }
        "assistant" => {
            if let Some(body) = &msg.message {
                if let Some(usage) = &body.usage {
                    let input_t = usage.input_tokens
                        + usage.cache_read_input_tokens
                        + usage.cache_creation_input_tokens;
                    let total = input_t + usage.output_tokens;
                    if total > 0 {
                        updates.push(json!({
                            "sessionId": session_id,
                            "_meta": { "totalTokens": total }
                        }));
                    }
                }
                if let Some(content) = &body.content {
                    for block in content {
                        if block.type_ == "tool_use" {
                            let id = block.id.clone().unwrap_or_default();
                            let name = block.name.clone().unwrap_or_default();
                            let raw_input = block.input.clone().unwrap_or(json!({}));
                            updates.push(json!({
                                "sessionId": session_id,
                                "update": {
                                    "sessionUpdate": "tool_call",
                                    "toolCallId": id,
                                    "title": name,
                                    "status": "pending",
                                    "rawInput": raw_input,
                                }
                            }));
                        }
                    }
                }
            }
        }
        "tool_result" | "user" => {
            if let Some(body) = &msg.message
                && let Some(content) = &body.content
            {
                for block in content {
                    if block.type_ == "tool_result" {
                        let id = block.tool_use_id.clone().unwrap_or_default();
                        let output = block.content.as_ref().map_or(String::new(), |v| {
                            v.as_str()
                                .map(str::to_string)
                                .unwrap_or_else(|| v.to_string())
                        });
                        updates.push(json!({
                            "sessionId": session_id,
                            "update": {
                                "sessionUpdate": "tool_call_update",
                                "toolCallId": id,
                                "title": "",
                                "status": "completed",
                                "content": [{
                                    "type": "content",
                                    "content": { "type": "text", "text": output }
                                }]
                            }
                        }));
                    }
                }
            }
        }
        "result" => {
            // Context window capacity from modelUsage when present.
            if let Some(cw) = msg
                .model_usage
                .as_ref()
                .and_then(|mu| mu.as_object())
                .and_then(|mu| mu.values().next())
                .and_then(|v| v["contextWindow"].as_u64())
            {
                // Profile usage is totalTokens; fold window only if client cares
                // via handshake — emit totalTokens unchanged shape when we have
                // nothing else. Skip empty result-only window; host already has
                // model windows from initialize for Claude aliases.
                let _ = cw;
            }
        }
        _ => {}
    }

    updates
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

    /// @spec harness/claude Profile-compatible event emission: Assistant text from Claude surfaces as profile content updates
    #[test]
    fn assistant_text_maps_to_agent_message_chunk() {
        let line = r#"{
            "type": "stream_event",
            "event": {
                "type": "content_block_delta",
                "delta": { "type": "text_delta", "text": "hello" }
            }
        }"#;
        let msg: ProtocolMsg = serde_json::from_str(line).unwrap();
        let updates = claude_line_to_updates(&msg, "sess-1");
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0]["update"]["sessionUpdate"], "agent_message_chunk");
        assert_eq!(updates[0]["update"]["content"]["text"], "hello");
        assert_eq!(updates[0]["sessionId"], "sess-1");
    }

    /// @spec harness/claude Profile-compatible event emission: Claude thinking surfaces as profile thought chunks
    #[test]
    fn claude_thinking_maps_to_agent_thought_chunk() {
        let line = r#"{
            "type": "stream_event",
            "event": {
                "type": "content_block_delta",
                "delta": { "type": "thinking_delta", "thinking": "let me reason" }
            }
        }"#;
        let msg: ProtocolMsg = serde_json::from_str(line).unwrap();
        let updates = claude_line_to_updates(&msg, "sess-1");
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0]["update"]["sessionUpdate"], "agent_thought_chunk");
        assert_eq!(updates[0]["update"]["content"]["text"], "let me reason");
        assert_eq!(updates[0]["sessionId"], "sess-1");
    }

    /// @spec harness/claude Profile-compatible event emission: A Claude tool call surfaces as profile tool use then result
    #[test]
    fn tool_use_and_result_map_to_profile_pair() {
        let use_line = r#"{
            "type": "assistant",
            "message": {
                "content": [{
                    "type": "tool_use",
                    "id": "call-9",
                    "name": "Read",
                    "input": { "path": "a.rs" }
                }]
            }
        }"#;
        let result_line = r#"{
            "type": "user",
            "message": {
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": "call-9",
                    "content": "fn main() {}"
                }]
            }
        }"#;
        let use_msg: ProtocolMsg = serde_json::from_str(use_line).unwrap();
        let result_msg: ProtocolMsg = serde_json::from_str(result_line).unwrap();

        let use_u = claude_line_to_updates(&use_msg, "s");
        assert_eq!(use_u[0]["update"]["sessionUpdate"], "tool_call");
        assert_eq!(use_u[0]["update"]["toolCallId"], "call-9");
        assert_eq!(use_u[0]["update"]["title"], "Read");
        assert_eq!(use_u[0]["update"]["rawInput"]["path"], "a.rs");

        let res_u = claude_line_to_updates(&result_msg, "s");
        assert_eq!(res_u[0]["update"]["sessionUpdate"], "tool_call_update");
        assert_eq!(res_u[0]["update"]["status"], "completed");
        assert_eq!(res_u[0]["update"]["toolCallId"], "call-9");
        assert_eq!(
            res_u[0]["update"]["content"][0]["content"]["text"],
            "fn main() {}"
        );
    }
}
