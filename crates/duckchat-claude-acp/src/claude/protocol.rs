//! Wire types for `claude -p --output-format stream-json` lines.

use serde::Deserialize;

/// Top-level protocol message from Claude Code stream-json output.
#[derive(Debug, Deserialize)]
pub struct ProtocolMsg {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub subtype: Option<String>,
    // stream_event
    pub event: Option<StreamEvent>,
    // assistant / tool_result / user
    pub message: Option<MessageBody>,
    // system
    #[allow(dead_code)]
    pub model: Option<String>,
    // result / system init — Claude uses snake_case `session_id`
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
    /// e.g. `text_delta`, `thinking_delta`, `signature_delta`.
    #[serde(rename = "type")]
    pub type_: Option<String>,
    /// Assistant text for `text_delta`.
    pub text: Option<String>,
    /// Reasoning/thinking for `thinking_delta`.
    pub thinking: Option<String>,
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

impl ProtocolMsg {
    /// Session id if present on this line (system init or result).
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn is_result(&self) -> bool {
        self.type_ == "result"
    }

    pub fn is_error_result(&self) -> bool {
        self.is_result() && self.is_error == Some(true)
    }

    pub fn error_message(&self) -> Option<String> {
        if !self.is_error_result() {
            return None;
        }
        self.result
            .as_ref()
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| Some("unknown error".into()))
    }
}
