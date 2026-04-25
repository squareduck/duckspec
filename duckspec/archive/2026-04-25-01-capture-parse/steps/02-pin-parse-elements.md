# Pin parse elements

Wire `@spec` backlinks for the 20 `parse/elements` scenarios. 3 of these are new tests
(added in `crates/duckpond/src/parse.rs` inline `mod tests`); the other 17 are already
covered by existing inline tests and only need the `@spec` comment added above the test.

## Prerequisites

- [ ] @step tighten-parse-marked-heading

## Context

The 28 inline tests in `crates/duckpond/src/parse.rs` cover most scenarios. For each
`@spec` task below, add a `// @spec parse/elements <Requirement>: <Scenario>` comment
immediately above any test that exercises the scenario (one is enough; the audit only
requires ≥1 backlink). For the three NEW scenarios marked below, write the inline test
first (~10 LOC each in the existing `mod tests` block), then add the comment.

The three new tests:

```rust
#[test]
fn unclosed_code_block_flushes_at_eof() {
    let elems = elements("```rust\nfoo");
    assert_eq!(elems.len(), 1);
    assert!(matches!(
        &elems[0],
        Element::Block { kind: BlockKind::CodeBlock, content, .. }
        if content.starts_with("```rust") && content.ends_with("foo")
    ));
}

#[test]
fn code_fence_with_info_string() {
    let elems = elements("```rust\nfn main() {}\n```");
    assert!(matches!(
        &elems[0],
        Element::Block { kind: BlockKind::CodeBlock, content, .. }
        if content.starts_with("```rust") && content.ends_with("```")
    ));
}

#[test]
fn block_quote_terminates_list_item() {
    let elems = elements("- item\n> quote");
    assert_eq!(elems.len(), 2);
    assert!(matches!(&elems[0], Element::ListItem { .. }));
    assert!(matches!(&elems[1], Element::BlockQuoteItem { .. }));
}
```

After all backlinks are in place, run `ds sync` to populate the cap spec's backlink lists,
then `ds audit` to confirm coverage.

## Tasks

- [x] 1. @spec parse/elements Element model and source spans: Empty input produces no elements

- [x] 2. @spec parse/elements Element model and source spans: Mixed content produces an ordered sequence of distinct kinds

- [x] 3. @spec parse/elements Element model and source spans: Element spans match byte offsets in the source

- [x] 4. @spec parse/elements Element model and source spans: Unclosed code block is flushed at end of input

- [x] 5. @spec parse/elements ATX heading classification: ATX headings at levels 1 through 6

- [x] 6. @spec parse/elements ATX heading classification: Hashes without a following space become paragraphs

- [x] 7. @spec parse/elements ATX heading classification: A heading terminates the preceding paragraph

- [x] 8. @spec parse/elements Paragraph aggregation: Multi-line paragraph stays a single block

- [x] 9. @spec parse/elements Paragraph aggregation: Blank line separates paragraphs

- [x] 10. @spec parse/elements Fenced code blocks: Code block preserves content verbatim

- [x] 11. @spec parse/elements Fenced code blocks: Code block preserves blank lines and info strings

- [x] 12. @spec parse/elements List item recognition: Bullet and numbered markers are distinguished

- [x] 13. @spec parse/elements List item recognition: Indent records nesting level

- [x] 14. @spec parse/elements List item recognition: Continuation lines aligned with content are absorbed

- [x] 15. @spec parse/elements List item recognition: Loose lists produce one ListItem per item

- [x] 16. @spec parse/elements List item recognition: Double-digit numbered markers are recognized

- [x] 17. @spec parse/elements List item recognition: List look-alikes fall back to paragraphs

- [x] 18. @spec parse/elements Block quote recognition: Each block quote line produces its own element

- [x] 19. @spec parse/elements Block quote recognition: Block-quote-formatted list-like content stays a block quote

- [x] 20. @spec parse/elements Block quote recognition: A block quote terminates a list item

## Outcomes

The original step file had each long `@spec` task wrapped across two unindented lines,
which the markdown parser does not recognize as a list-item continuation. `ds format`
silently drops the orphaned second line, leaving truncated scenario names that fail to
resolve in `ds audit`. Tasks were rewritten on single lines to match the cap spec
scenario names. Future `/ds-step` runs producing long scenario titles should keep each
task on a single line (or use ≥2-space continuation indent).
