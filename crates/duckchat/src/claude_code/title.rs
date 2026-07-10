//! One-shot summariser that asks Claude Haiku for a short session title.
//!
//! Deliberately avoids `run_turn`: no session resume, no tools, no permission
//! prompts, no stream-json. Prompt framing is shared via [`crate::title`];
//! process spawn is the cold [`super::runtime::ClaudeOneshotRuntime`].

use std::path::Path;

use crate::error::Error;
use crate::request::TitleRequest;
use crate::title::{build_title_prompt, clean_title};

pub async fn title_summary(req: TitleRequest, working_dir: &Path) -> Result<String, Error> {
    use crate::runtime::{OneshotKind, OneshotRuntime};

    use super::runtime::ClaudeOneshotRuntime;

    let prompt = build_title_prompt(&req);
    let mut rt = ClaudeOneshotRuntime::new(working_dir);
    let raw = rt.prompt(OneshotKind::Title, prompt).await?;
    Ok(clean_title(&raw))
}
