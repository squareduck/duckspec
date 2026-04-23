//! Wire types for `claude -p --output-format stream-json` and the mapping
//! from protocol messages to provider-neutral [`AgentEvent`]s.

use serde::Deserialize;

use crate::event::{AgentEvent, Usage};

/// Top-level protocol message from `claude -p --output-format stream-json`.
#[derive(Debug, Deserialize)]
pub struct ProtocolMsg {
    #[serde(rename = "type")]
    pub type_: String,
    // stream_event
    pub event: Option<StreamEvent>,
    // assistant / tool_result / user
    pub message: Option<MessageBody>,
    // system
    pub model: Option<String>,
    // result
    pub session_id: Option<String>,
    #[serde(rename = "modelUsage")]
    pub model_usage: Option<serde_json::Value>,
    pub is_error: Option<bool>,
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct StreamEvent {
    #[serde(rename = "type")]
    pub type_: String,
    pub delta: Option<DeltaBlock>,
}

#[derive(Debug, Deserialize)]
pub struct DeltaBlock {
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessageBody {
    pub content: Option<Vec<ContentBlock>>,
    pub usage: Option<UsageBlock>,
}

#[derive(Debug, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub type_: String,
    // tool_use
    pub id: Option<String>,
    pub name: Option<String>,
    pub input: Option<serde_json::Value>,
    // tool_result
    pub tool_use_id: Option<String>,
    pub content: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsageBlock {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
}

/// Parse a single protocol line into zero or more `AgentEvent`s.
pub fn parse_protocol_line(msg: &ProtocolMsg) -> Vec<AgentEvent> {
    let mut events = Vec::new();

    match msg.type_.as_str() {
        "stream_event" => {
            if let Some(event) = &msg.event
                && event.type_ == "content_block_delta"
                && let Some(delta) = &event.delta
                && let Some(text) = &delta.text
            {
                events.push(AgentEvent::ContentDelta { text: text.clone() });
            }
        }
        "assistant" => {
            if let Some(body) = &msg.message {
                if let Some(usage) = &body.usage {
                    // Assistant messages report per-request usage. Summing all
                    // three input fields gives the prompt size at this turn —
                    // i.e. current context-window fill. (Do NOT use the `result`
                    // message for this: its usage is cumulative across every
                    // internal model call, which inflates `cache_read` N-fold
                    // when the agent loops through tool use.)
                    let input_t = (usage.input_tokens
                        + usage.cache_read_input_tokens
                        + usage.cache_creation_input_tokens)
                        as usize;
                    let output_t = usage.output_tokens as usize;
                    if input_t > 0 || output_t > 0 {
                        events.push(AgentEvent::UsageUpdate(Usage {
                            input_tokens: Some(input_t),
                            output_tokens: Some(output_t),
                            context_window: None,
                        }));
                    }
                }
                if let Some(content) = &body.content {
                    for block in content {
                        if block.type_ == "tool_use" {
                            events.push(AgentEvent::ToolUse {
                                id: block.id.clone().unwrap_or_default(),
                                name: block.name.clone().unwrap_or_default(),
                                input: block
                                    .input
                                    .as_ref()
                                    .map_or(String::new(), |v| v.to_string()),
                            });
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
                        let output = block.content.as_ref().map_or(String::new(), |v| {
                            v.as_str().map_or_else(|| v.to_string(), |s| s.to_string())
                        });
                        events.push(AgentEvent::ToolResult {
                            id: block.tool_use_id.clone().unwrap_or_default(),
                            name: String::new(),
                            output,
                        });
                    }
                }
            }
        }
        "system" => {
            if let Some(model) = &msg.model {
                events.push(AgentEvent::ModelUpdate {
                    model: model.clone(),
                });
            }
        }
        "result" => {
            // Only propagate the context-window capacity here. `msg.usage` on
            // result messages is cumulative across every internal model call
            // in the turn (with prompt caching that multiplies `cache_read`
            // several-fold), so it's unusable as a current-prompt-size signal.
            let context_window = msg
                .model_usage
                .as_ref()
                .and_then(|mu| mu.as_object())
                .and_then(|mu| mu.values().next())
                .and_then(|v| v["contextWindow"].as_u64())
                .map(|v| v as usize);

            if let Some(cw) = context_window {
                events.push(AgentEvent::UsageUpdate(Usage {
                    input_tokens: None,
                    output_tokens: None,
                    context_window: Some(cw),
                }));
            }
        }
        _ => {}
    }

    events
}
