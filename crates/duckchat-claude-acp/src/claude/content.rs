//! ACP prompt content blocks → Claude / Anthropic user content blocks.

use serde_json::{Value, json};

/// Translate ACP `params.prompt` content blocks into Claude user `content`.
///
/// Supported ACP block types:
/// - `text` → `{ "type": "text", "text": … }`
/// - `image` with ACP resource / data shapes → Anthropic image source block
///
/// Unknown or empty blocks are skipped. If nothing remains, a single empty
/// text block is returned so Claude always receives a well-formed message.
pub fn acp_prompt_to_claude_content(params: &Value) -> Vec<Value> {
    let Some(blocks) = params.get("prompt").and_then(Value::as_array) else {
        return vec![json!({ "type": "text", "text": "" })];
    };

    let mut out = Vec::new();
    for block in blocks {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    out.push(json!({ "type": "text", "text": text }));
                }
            }
            Some("image") => {
                if let Some(img) = encode_acp_image(block) {
                    out.push(img);
                }
            }
            _ => {}
        }
    }

    if out.is_empty() {
        out.push(json!({ "type": "text", "text": "" }));
    }
    out
}

/// ACP image blocks arrive as either:
/// - `{ "type": "image", "data": "<b64>", "mimeType": "image/png" }` (common)
/// - `{ "type": "image", "source": { "type": "base64", "media_type": …, "data": … } }`
fn encode_acp_image(block: &Value) -> Option<Value> {
    if let Some(source) = block.get("source") {
        let media = source
            .get("media_type")
            .or_else(|| source.get("mediaType"))
            .and_then(Value::as_str)?;
        let data = source.get("data").and_then(Value::as_str)?;
        return Some(json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": media,
                "data": data,
            }
        }));
    }

    let media = block
        .get("mimeType")
        .or_else(|| block.get("mediaType"))
        .and_then(Value::as_str)
        .unwrap_or("image/png");
    if let Some(data) = block.get("data").and_then(Value::as_str) {
        return Some(json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": media,
                "data": data,
            }
        }));
    }

    // Raw bytes are not on the wire; ignore incomplete image blocks.
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_and_image_blocks_translate() {
        let params = json!({
            "prompt": [
                { "type": "text", "text": "look" },
                {
                    "type": "image",
                    "mimeType": "image/png",
                    "data": "AQID"
                }
            ]
        });
        let blocks = acp_prompt_to_claude_content(&params);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0]["type"], "text");
        assert_eq!(blocks[0]["text"], "look");
        assert_eq!(blocks[1]["type"], "image");
        assert_eq!(blocks[1]["source"]["media_type"], "image/png");
        assert_eq!(blocks[1]["source"]["data"], "AQID");
    }

    #[test]
    fn empty_prompt_yields_empty_text_block() {
        let blocks = acp_prompt_to_claude_content(&json!({}));
        assert_eq!(blocks, vec![json!({ "type": "text", "text": "" })]);
    }
}
