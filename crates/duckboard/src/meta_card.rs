//! Duckboard-local recognition of chat `write` / `next` meta cards.
//!
//! Line-oriented scan only — does not use duckpond parsers.

/// Maximum trailing-next actions extracted from one card.
pub const MAX_NEXT_ACTIONS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaCardKind {
    Write,
    Next,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaCard {
    pub kind: MetaCardKind,
    /// Inclusive 0-based start line of the quote run.
    pub line_start: usize,
    /// Inclusive 0-based end line of the quote run.
    pub line_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextAction {
    /// Exact send text from the first `` `token` `` on the line.
    pub send: String,
    /// Optional UI label after the token (not sent).
    pub reason: Option<String>,
}

/// Parse all meta cards in `source` (write and next).
pub fn parse_meta_cards(source: &str) -> Vec<MetaCard> {
    let lines: Vec<&str> = source.lines().collect();
    let mut cards = Vec::new();
    let mut i = 0;
    let mut in_fence = false;

    while i < lines.len() {
        let line = lines[i];
        if is_fence_line(line) {
            in_fence = !in_fence;
            i += 1;
            continue;
        }
        if in_fence || !is_blockquote_line(line) {
            i += 1;
            continue;
        }

        // Start of a maximal blockquote run (outside fences).
        let run_start = i;
        while i < lines.len() {
            let l = lines[i];
            if is_fence_line(l) {
                // Fence ends the quote run; fence state flips on the fence line
                // after we leave the run.
                break;
            }
            if !is_blockquote_line(l) {
                break;
            }
            i += 1;
        }
        let run_end = i - 1; // inclusive; run was non-empty

        if let Some(kind) = card_kind_for_run(&lines[run_start..=run_end]) {
            cards.push(MetaCard {
                kind,
                line_start: run_start,
                line_end: run_end,
            });
        }
        // Do not advance i again — loop will process fence / next line at i.
    }

    cards
}

/// Per-line flags: `true` when that 0-based line index falls in any meta card
/// inclusive range (`write` or `next`). Length matches `source.lines().count()`
/// (0 when `source` is empty).
pub fn meta_card_line_flags(source: &str) -> Vec<bool> {
    let line_count = source.lines().count();
    let mut flags = vec![false; line_count];
    for card in parse_meta_cards(source) {
        for i in card.line_start..=card.line_end {
            if let Some(slot) = flags.get_mut(i) {
                *slot = true;
            }
        }
    }
    flags
}

/// Trailing `next` card actions only (cap [`MAX_NEXT_ACTIONS`]).
pub fn trailing_next_actions(source: &str) -> Vec<NextAction> {
    let lines: Vec<&str> = source.lines().collect();
    let cards = parse_meta_cards(source);
    let Some(card) = cards
        .iter()
        .rev()
        .find(|c| c.kind == MetaCardKind::Next && is_trailing_card(c, &lines))
    else {
        return Vec::new();
    };

    let mut actions = Vec::new();
    for line in &lines[card.line_start..=card.line_end] {
        let content = blockquote_content(line);
        let trimmed = content.trim();
        // Kind line and blank quote lines are not actions.
        if trimmed.is_empty() || trimmed == "**next**" {
            continue;
        }
        if let Some(action) = action_from_body_line(content) {
            actions.push(action);
            if actions.len() >= MAX_NEXT_ACTIONS {
                break;
            }
        }
    }
    actions
}

fn is_trailing_card(card: &MetaCard, lines: &[&str]) -> bool {
    let last_non_empty = lines.iter().rposition(|l| !l.trim().is_empty());
    match last_non_empty {
        None => false,
        Some(idx) => card.line_end >= idx,
    }
}

fn card_kind_for_run(run_lines: &[&str]) -> Option<MetaCardKind> {
    for line in run_lines {
        let content = blockquote_content(line);
        let trimmed = content.trim();
        if trimmed.is_empty() {
            continue;
        }
        return match trimmed {
            "**write**" => Some(MetaCardKind::Write),
            "**next**" => Some(MetaCardKind::Next),
            _ => None,
        };
    }
    None
}

/// Line is a blockquote: leading whitespace, then `>` alone or `>` + space.
fn is_blockquote_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed == ">" || trimmed.starts_with("> ")
}

fn blockquote_content(line: &str) -> &str {
    let trimmed = line.trim_start();
    if trimmed == ">" {
        ""
    } else {
        // Caller only passes blockquote lines; bare `>` handled above.
        trimmed.strip_prefix("> ").unwrap_or_default()
    }
}

/// Fence open/close: a line whose trimmed form starts with ``` (CommonMark-ish).
fn is_fence_line(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

/// First `` `token` `` on the line → send; remainder after span → reason.
fn action_from_body_line(content: &str) -> Option<NextAction> {
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'`' {
                j += 1;
            }
            if j >= bytes.len() {
                return None; // unclosed
            }
            let send = content[start..j].to_string();
            if send.is_empty() {
                // Empty token — skip this span, keep looking? Spec says first
                // inline code span; empty send is still a span. Treat empty as
                // skip of the line for usefulness (no send text).
                return None;
            }
            let after = content[j + 1..].trim();
            let reason = if after.is_empty() {
                None
            } else {
                Some(after.to_string())
            };
            return Some(NextAction { send, reason });
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // @spec chat/meta-cards Card recognition: Known-kind quote run yields a card with inclusive line range
    #[test]
    fn known_kind_quote_run_yields_card_with_inclusive_line_range() {
        // GIVEN assistant markdown whose only blockquote run starts with
        // `> **next**` and continues with two more blockquote body lines
        let src = "\
intro

> **next**
>
> `confirm`  do it
";
        // WHEN meta cards are parsed from that markdown
        let cards = parse_meta_cards(src);
        // THEN exactly one meta card is produced
        assert_eq!(cards.len(), 1);
        // AND the card's kind is next
        assert_eq!(cards[0].kind, MetaCardKind::Next);
        // AND the card's inclusive line range covers exactly those three
        // blockquote lines (lines 2,3,4 after intro blank: indices 2..=4)
        // Lines: 0=intro, 1=empty, 2=> **next**, 3=>, 4=> `confirm`...
        assert_eq!(cards[0].line_start, 2);
        assert_eq!(cards[0].line_end, 4);
    }

    // @spec chat/meta-cards Card recognition: Ordinary blockquote is not a meta card
    #[test]
    fn ordinary_blockquote_is_not_a_meta_card() {
        // GIVEN assistant markdown with a blockquote whose first non-empty
        // content is ordinary prose
        let src = "\
> This is a normal quote
> still quoting
";
        // WHEN meta cards are parsed
        let cards = parse_meta_cards(src);
        // THEN no meta card is produced for that run
        assert!(cards.is_empty());
    }

    // @spec chat/meta-cards Card recognition: Known-kind line inside a fenced code block is not a meta card
    #[test]
    fn known_kind_line_inside_fenced_code_block_is_not_a_meta_card() {
        // GIVEN assistant markdown that places `> **next**` only inside a fence
        let src = "\
```markdown
> **next**
>
> `confirm`  example
```
";
        // WHEN meta cards are parsed
        let cards = parse_meta_cards(src);
        // THEN no meta card is produced from that line
        assert!(cards.is_empty());
    }

    // @spec chat/meta-cards Trailing next actions: Trailing next card yields ordered send tokens
    #[test]
    fn trailing_next_card_yields_ordered_send_tokens() {
        // GIVEN markdown ending with a next card and two token body lines
        let src = "\
Some prose.

> **next**
>
> `/ds-propose`  draft proposal
> `/ds-design`  design the approach
";
        // WHEN trailing next actions are extracted
        let actions = trailing_next_actions(src);
        // THEN exactly those two send texts in source order
        assert_eq!(
            actions.iter().map(|a| a.send.as_str()).collect::<Vec<_>>(),
            vec!["/ds-propose", "/ds-design"]
        );
    }

    // @spec chat/meta-cards Trailing next actions: Non-trailing next card yields no actions
    #[test]
    fn non_trailing_next_card_yields_no_actions() {
        // GIVEN a next card followed by non-blank non-blockquote content
        let src = "\
> **next**
>
> `confirm`  write this

More prose after the card.
";
        // WHEN trailing next actions are extracted
        let actions = trailing_next_actions(src);
        // THEN the action list is empty
        assert!(actions.is_empty());
    }

    // @spec chat/meta-cards Trailing next actions: Actions capped at three in source order
    #[test]
    fn actions_capped_at_three_in_source_order() {
        // GIVEN trailing next with four token-bearing body lines
        let src = "\
> **next**
>
> `a`  one
> `b`  two
> `c`  three
> `d`  four
";
        // WHEN trailing next actions are extracted
        let actions = trailing_next_actions(src);
        // THEN exactly three entries — first three in source order
        assert_eq!(
            actions.iter().map(|a| a.send.as_str()).collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
    }

    // @spec chat/meta-cards Trailing next actions: Body line without a token is skipped
    #[test]
    fn body_line_without_a_token_is_skipped() {
        // GIVEN a body line with no code span between two token lines
        let src = "\
> **next**
>
> `first`  ok
> plain reason line
> `second`  also
";
        // WHEN trailing next actions are extracted
        let actions = trailing_next_actions(src);
        // THEN exactly two send texts from the token-bearing lines
        assert_eq!(
            actions.iter().map(|a| a.send.as_str()).collect::<Vec<_>>(),
            vec!["first", "second"]
        );
    }

    // @spec chat/meta-cards Trailing next actions: Reason after the token is not part of send text
    #[test]
    fn reason_after_the_token_is_not_part_of_send_text() {
        // GIVEN first span `confirm` with a non-empty reason after
        let src = "\
> **next**
>
> `confirm`  write this proposal
";
        // WHEN trailing next actions are extracted
        let actions = trailing_next_actions(src);
        assert_eq!(actions.len(), 1);
        // THEN send text is exactly confirm
        assert_eq!(actions[0].send, "confirm");
        // AND reason text is not included in the send text
        assert!(!actions[0].send.contains("write"));
        assert_eq!(actions[0].reason.as_deref(), Some("write this proposal"));
    }

    // @spec chat/transcript Meta-card line background: Meta-card lines on an Answer get meta-card background
    #[test]
    fn meta_card_lines_on_an_answer_get_meta_card_background() {
        // GIVEN an Answer whose source contains a recognized next meta card
        let src = "\
Done.

> **next**
>
> `/ds-spec`  write specs
";
        // WHEN display line backgrounds are prepared
        let flags = meta_card_line_flags(src);
        let cards = parse_meta_cards(src);
        assert_eq!(cards.len(), 1);
        // THEN every line index in that range has a meta-card background
        for i in cards[0].line_start..=cards[0].line_end {
            assert!(
                flags[i],
                "line {i} should have meta-card background (flags={flags:?})"
            );
        }
    }

    // @spec chat/transcript Meta-card line background: Non-meta lines on the same Answer do not get meta-card background
    #[test]
    fn non_meta_lines_on_the_same_answer_do_not_get_meta_card_background() {
        // GIVEN ordinary prose before a recognized meta card
        let src = "\
Ordinary prose line one.
Ordinary prose line two.

> **next**
>
> `confirm`  ok
";
        // WHEN display line backgrounds are prepared
        let flags = meta_card_line_flags(src);
        // THEN ordinary prose lines (0 and 1) do not have meta-card background
        assert!(!flags[0], "prose line 0 must not be tinted");
        assert!(!flags[1], "prose line 1 must not be tinted");
        // blank line 2 is outside the card
        assert!(!flags[2], "blank before card must not be tinted");
        // card lines are tinted
        assert!(flags[3] && flags[4] && flags[5]);
    }
}
