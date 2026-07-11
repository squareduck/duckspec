//! Pure translation of ACP profile `session/update` notifications into
//! duckchat's neutral [`AgentEvent`] stream.
//!
//! [`map_update`] is deliberately a pure function of the raw `session/update`
//! `params` plus the active model's context window, so it can be unit-tested
//! against recorded JSON without a live agent process. The turn layer wires it
//! into the read loop (see [`super::turn::AcpTurn::prompt_events`]) and maps
//! the terminal `stop_reason` onto `TurnComplete`/`Error` itself.

use serde_json::Value;

use crate::event::{AgentEvent, Usage};

/// Translate one ACP profile `session/update` `params` payload into a neutral
/// [`AgentEvent`], or `None` when the update carries nothing we surface.
///
/// `context_window` is the active model's window discovered during the
/// handshake; it is folded into every emitted [`AgentEvent::UsageUpdate`] so
/// the usage meter has a denominator. It comes from the model, never from an
/// incidental value in the update itself.
pub fn map_update(params: &Value, context_window: Option<usize>) -> Option<AgentEvent> {
    if let Some(update) = params.get("update") {
        match update.get("sessionUpdate").and_then(Value::as_str) {
            Some("agent_message_chunk") => {
                return Some(AgentEvent::ContentDelta {
                    text: chunk_text(update)?,
                });
            }
            Some("agent_thought_chunk") => {
                return Some(AgentEvent::ReasoningDelta {
                    text: chunk_text(update)?,
                });
            }
            Some("tool_call") => {
                return Some(AgentEvent::ToolUse {
                    id: str_field(update, "toolCallId")?,
                    name: title(update),
                    // `rawInput` is an arbitrary JSON object; carry it as its
                    // compact JSON serialization.
                    input: update
                        .get("rawInput")
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                });
            }
            Some("tool_call_update") => {
                // Only a *completed* update carries a result; earlier
                // in-progress updates (status `pending`/`in_progress`) are
                // skipped so a call surfaces as exactly one use + one result.
                if update.get("status").and_then(Value::as_str) == Some("completed") {
                    return Some(AgentEvent::ToolResult {
                        id: str_field(update, "toolCallId")?,
                        name: title(update),
                        output: tool_output(update),
                    });
                }
            }
            _ => {}
        }
    }

    // Usage telemetry rides on `_meta.totalTokens`. Reported as `input_tokens`
    // (with no output delta) so duckboard's meter — which sums input + output —
    // shows the running total against the model's window.
    if let Some(total) = params.pointer("/_meta/totalTokens").and_then(Value::as_u64) {
        return Some(AgentEvent::UsageUpdate(Usage {
            input_tokens: Some(total as usize),
            output_tokens: None,
            context_window,
        }));
    }

    None
}

/// Text of an `agent_message_chunk` / `agent_thought_chunk`: their `content` is
/// a single `{ "type": "text", "text": … }` block.
fn chunk_text(update: &Value) -> Option<String> {
    update
        .pointer("/content/text")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// A tool call's human-readable name, from its `title`. Empty when absent.
fn title(update: &Value) -> String {
    update
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

/// Flatten a `tool_call_update`'s `content` array into one string. Each entry
/// is an ACP content block (`{ "type": "content", "content": { "type": "text",
/// "text": … } }`); non-text blocks are ignored.
fn tool_output(update: &Value) -> String {
    let Some(items) = update.get("content").and_then(Value::as_array) else {
        return String::new();
    };
    let mut out = String::new();
    for item in items {
        if let Some(text) = item
            .pointer("/content/text")
            .and_then(Value::as_str)
            .or_else(|| item.get("text").and_then(Value::as_str))
        {
            out.push_str(text);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// @spec harness/acp-client Profile event translation: Assistant text and reasoning surface on distinct channels
    #[test]
    fn message_and_thought_map_to_distinct_channels() {
        let message = json!({
            "sessionId": "s1",
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": "hello" }
            }
        });
        let thought = json!({
            "sessionId": "s1",
            "update": {
                "sessionUpdate": "agent_thought_chunk",
                "content": { "type": "text", "text": "thinking" }
            }
        });

        // Assistant text surfaces as a content event.
        match map_update(&message, None) {
            Some(AgentEvent::ContentDelta { text }) => assert_eq!(text, "hello"),
            other => panic!("expected ContentDelta, got {other:?}"),
        }
        // Reasoning surfaces on a separate reasoning channel.
        match map_update(&thought, None) {
            Some(AgentEvent::ReasoningDelta { text }) => assert_eq!(text, "thinking"),
            other => panic!("expected ReasoningDelta, got {other:?}"),
        }
    }

    /// @spec harness/acp-client Profile event translation: A tool call surfaces as a use then a matching result
    #[test]
    fn tool_call_maps_to_use_then_matching_result() {
        let call = json!({
            "sessionId": "s1",
            "update": {
                "sessionUpdate": "tool_call",
                "toolCallId": "call-1",
                "title": "read_file",
                "status": "pending",
                "rawInput": { "path": "foo.rs" }
            }
        });
        let done = json!({
            "sessionId": "s1",
            "update": {
                "sessionUpdate": "tool_call_update",
                "toolCallId": "call-1",
                "title": "read_file",
                "status": "completed",
                "content": [
                    { "type": "content", "content": { "type": "text", "text": "fn main() {}" } }
                ]
            }
        });

        let (use_id, name, input) = match map_update(&call, None) {
            Some(AgentEvent::ToolUse { id, name, input }) => (id, name, input),
            other => panic!("expected ToolUse, got {other:?}"),
        };
        assert_eq!(use_id, "call-1");
        assert_eq!(name, "read_file");
        assert!(input.contains("foo.rs"));

        let (result_id, output) = match map_update(&done, None) {
            Some(AgentEvent::ToolResult { id, output, .. }) => (id, output),
            other => panic!("expected ToolResult, got {other:?}"),
        };
        // The result carries the same call id, linking it back to the use.
        assert_eq!(result_id, use_id);
        assert_eq!(output, "fn main() {}");
    }

    /// @spec harness/acp-client Profile event translation: A usage update carries used tokens and the model's context window
    #[test]
    fn usage_update_carries_tokens_and_window() {
        let update = json!({
            "sessionId": "s1",
            "_meta": { "totalTokens": 4096 }
        });

        match map_update(&update, Some(128_000)) {
            Some(AgentEvent::UsageUpdate(usage)) => {
                // The running total surfaces as the meter's used-token numerator.
                let used = usage.input_tokens.unwrap_or(0) + usage.output_tokens.unwrap_or(0);
                assert_eq!(used, 4096);
                // The denominator is the active model's window, taken from the
                // handshake — not from anything in the update.
                assert_eq!(usage.context_window, Some(128_000));
            }
            other => panic!("expected UsageUpdate, got {other:?}"),
        }
    }
}
