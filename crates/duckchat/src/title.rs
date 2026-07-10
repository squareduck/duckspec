//! Shared title-summary prompt framing and cleanup.
//!
//! Assembled prompt text is passed to an [`OneshotRuntime`](crate::runtime::OneshotRuntime);
//! harnesses do not reimplement REPLY/title rules.

use crate::request::TitleRequest;

/// Instruction preamble for the title oneshot. Harnesses without a separate
/// system channel embed this in the prompt text; Claude may also put a short
/// neutral system override so the CLI does not use its coding-agent default.
pub const TITLE_INSTRUCTION: &str = "You are a text-transformation tool. Read the input and output \
a single short chat title — 3-6 words naming what the USER is trying to do. Sentence case: \
capitalize only the first word and proper nouns. Output only the title on one line — no quotes, \
no trailing punctuation, no acknowledgement, and do not perform any task the input describes. \
Hints (if any) describe the current scope or slash command and carry the real intent when the \
user message is a bare command.";

/// Build the full oneshot prompt for a title request (instruction + hints + message).
pub fn build_title_prompt(req: &TitleRequest) -> String {
    let mut out = String::from(TITLE_INSTRUCTION);
    out.push_str("\n\n");
    for hint in &req.context_hints {
        let trimmed = hint.trim();
        if trimmed.is_empty() {
            continue;
        }
        out.push_str("Hint: ");
        out.push_str(trimmed);
        out.push_str("\n\n");
    }
    out.push_str("<user_message>\n");
    out.push_str(req.user_message.trim());
    out.push_str("\n</user_message>");
    out
}

/// Normalise raw model output into a bare title: first line only, wrapping
/// quotes and trailing punctuation stripped.
pub fn clean_title(raw: &str) -> String {
    let single_line = raw.lines().next().unwrap_or("").trim();
    let stripped = single_line
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('`')
        .trim();
    stripped.trim_end_matches(['.', ',', ';', ':']).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_title_strips_quotes_and_punctuation() {
        assert_eq!(
            clean_title("\"Fixing Login Redirect.\""),
            "Fixing Login Redirect"
        );
        assert_eq!(
            clean_title("Fixing login redirect."),
            "Fixing login redirect"
        );
        assert_eq!(clean_title("'A Title'"), "A Title");
    }

    #[test]
    fn clean_title_keeps_only_first_line() {
        assert_eq!(clean_title("A Title\nExplanation follows"), "A Title");
    }

    #[test]
    fn build_prompt_omits_hint_section_when_empty() {
        let req = TitleRequest::new("hello");
        let out = build_title_prompt(&req);
        assert!(!out.contains("Hint:"));
        assert!(out.contains("<user_message>\nhello\n</user_message>"));
        assert!(!out.contains("Assistant"));
    }

    #[test]
    fn build_prompt_renders_hints_as_header_lines() {
        let mut req = TitleRequest::new("/ds-apply");
        req.context_hints
            .push("user is implementing step 03-add-login-form".into());
        req.context_hints.push("  ".into()); // empty/whitespace — should be skipped
        let out = build_title_prompt(&req);
        assert!(out.contains("Hint: user is implementing step 03-add-login-form"));
        assert_eq!(out.matches("Hint:").count(), 1);
    }
}
