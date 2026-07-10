//! One-shot reply suggestions via Claude Haiku (same cheap model as titles).

use std::path::Path;

use crate::error::Error;
use crate::reply_suggest::{
    REPLY_SUGGEST_INSTRUCTION, build_reply_suggest_prompt, parse_replies, should_skip_model,
};
use crate::request::ReplySuggestionRequest;
use crate::runtime::{OneshotKind, OneshotRuntime};

use super::runtime::ClaudeOneshotRuntime;

pub async fn reply_suggestions(
    req: ReplySuggestionRequest,
    working_dir: &Path,
) -> Result<Vec<String>, Error> {
    if should_skip_model(&req) {
        return Ok(Vec::new());
    }

    let body = build_reply_suggest_prompt(&req);
    let prompt = format!("{REPLY_SUGGEST_INSTRUCTION}\n\n{body}");
    let mut rt = ClaudeOneshotRuntime::new(working_dir);
    let raw = rt.prompt(OneshotKind::ReplySuggest, prompt).await?;
    Ok(parse_replies(&raw))
}
