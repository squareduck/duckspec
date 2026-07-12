//! ACP prompt content blocks → App Server turn input.
//!
//! Text maps 1:1. Image blocks (base64) are written to per-turn temp files and
//! emitted as `{ type: "localImage", path }`. Call [`TurnInput::cleanup`] when
//! the turn ends (or on cancel / drop).

use std::path::PathBuf;

use base64::Engine as _;
use serde_json::{Value, json};

/// Turn input blocks plus temp files that must be removed after the turn.
#[derive(Debug, Default)]
pub struct TurnInput {
    pub blocks: Vec<Value>,
    temp_paths: Vec<PathBuf>,
}

impl TurnInput {
    /// Delete any localImage temp files written for this turn.
    pub fn cleanup(&mut self) {
        for path in self.temp_paths.drain(..) {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl Drop for TurnInput {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Translate ACP `params.prompt` into App Server `turn/start` input.
///
/// Supported ACP block types:
/// - `text` → `{ "type": "text", "text": … }`
/// - `image` with base64 data → temp file + `{ "type": "localImage", "path": … }`
pub fn acp_prompt_to_turn_input(params: &Value) -> TurnInput {
    let Some(blocks) = params.get("prompt").and_then(Value::as_array) else {
        return TurnInput {
            blocks: vec![json!({ "type": "text", "text": "" })],
            temp_paths: Vec::new(),
        };
    };

    let mut out = TurnInput::default();
    for block in blocks {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    out.blocks.push(json!({ "type": "text", "text": text }));
                }
            }
            Some("image") => {
                if let Some((path, temp)) = write_image_temp(block) {
                    out.blocks
                        .push(json!({ "type": "localImage", "path": path }));
                    out.temp_paths.push(temp);
                }
            }
            _ => {}
        }
    }

    if out.blocks.is_empty() {
        out.blocks.push(json!({ "type": "text", "text": "" }));
    }
    out
}

/// Decode ACP image base64 and write bytes to a unique temp path.
/// Returns `(path_string_for_wire, path_for_cleanup)`.
fn write_image_temp(block: &Value) -> Option<(String, PathBuf)> {
    let (media, data_b64) = image_parts(block)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_b64.as_bytes())
        .ok()?;
    let ext = extension_for_media(media);
    let path = std::env::temp_dir().join(format!(
        "duckchat-codex-img-{}-{}.{}",
        std::process::id(),
        unique_stamp(),
        ext
    ));
    std::fs::write(&path, &bytes).ok()?;
    let path_str = path.to_string_lossy().into_owned();
    Some((path_str, path))
}

fn image_parts(block: &Value) -> Option<(&str, &str)> {
    if let Some(source) = block.get("source") {
        let media = source
            .get("media_type")
            .or_else(|| source.get("mediaType"))
            .and_then(Value::as_str)
            .unwrap_or("image/png");
        let data = source.get("data").and_then(Value::as_str)?;
        return Some((media, data));
    }
    let media = block
        .get("mimeType")
        .or_else(|| block.get("mediaType"))
        .and_then(Value::as_str)
        .unwrap_or("image/png");
    let data = block.get("data").and_then(Value::as_str)?;
    Some((media, data))
}

fn extension_for_media(media: &str) -> &'static str {
    match media {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "png",
    }
}

fn unique_stamp() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    N.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// @spec harness/openai-codex Prompt attachments: A resolved image attachment is delivered as a local image input on the turn
    #[test]
    fn resolved_image_is_local_image_input() {
        // GIVEN ACP image block (host already resolved attach: → image bytes)
        let params = json!({
            "prompt": [{
                "type": "image",
                "mimeType": "image/png",
                "data": "AQIDBA==" // bytes 1,2,3,4
            }]
        });
        // WHEN assembling turn input
        let mut input = acp_prompt_to_turn_input(&params);
        // THEN local image input for that attachment
        assert_eq!(input.blocks.len(), 1);
        assert_eq!(input.blocks[0]["type"], "localImage");
        let path = input.blocks[0]["path"].as_str().unwrap().to_string();
        let written = fs::read(&path).unwrap();
        assert_eq!(written, vec![1, 2, 3, 4]);
        input.cleanup();
        assert!(!std::path::Path::new(&path).exists());
    }

    /// @spec harness/openai-codex Prompt attachments: Surrounding text is preserved as text inputs
    #[test]
    fn surrounding_text_preserved_as_text_inputs() {
        let params = json!({
            "prompt": [
                { "type": "text", "text": "before " },
                {
                    "type": "image",
                    "mimeType": "image/png",
                    "data": "AQID"
                },
                { "type": "text", "text": " after" }
            ]
        });
        let mut input = acp_prompt_to_turn_input(&params);
        assert_eq!(input.blocks.len(), 3);
        assert_eq!(input.blocks[0], json!({ "type": "text", "text": "before " }));
        assert_eq!(input.blocks[1]["type"], "localImage");
        assert_eq!(input.blocks[2], json!({ "type": "text", "text": " after" }));
        input.cleanup();
    }

    /// @spec harness/openai-codex Prompt attachments: An unresolved attach marker is left literal
    #[test]
    fn unresolved_attach_marker_left_literal() {
        // Host leaves unresolved `[label](attach:id)` as plain text in the prompt.
        let params = json!({
            "prompt": [{
                "type": "text",
                "text": "see [clip](attach:missing) please"
            }]
        });
        let input = acp_prompt_to_turn_input(&params);
        assert_eq!(input.blocks.len(), 1);
        assert_eq!(
            input.blocks[0],
            json!({
                "type": "text",
                "text": "see [clip](attach:missing) please"
            })
        );
    }
}
