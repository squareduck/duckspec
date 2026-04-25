# Markdown elements

Layer 1 of the parsing pipeline. Tokenizes a markdown source string into a flat sequence
of `Element` values (Heading, Block, ListItem, BlockQuoteItem) with byte-offset spans.

The element stream is the substrate every Layer 2 artifact parser consumes. Layer 1 owns
markdown mechanics (line classification, paragraph aggregation, fenced-block tracking,
span bookkeeping) and is infallible: any input produces a valid sequence. Structural
validation — H1 presence, requirement ordering, GWT clauses, marker rules — is the job of
Layer 2.

## Requirement: Element model and source spans

The system SHALL parse a markdown source string into a flat ordered sequence of `Element`
values. Each element is exactly one of: `Heading` (with level 1–6 and text content),
`Block` of variant `Paragraph` or `CodeBlock`, `ListItem` (with `Bullet` or `Numbered`
marker and indent), or `BlockQuoteItem`. Every element SHALL carry a `Span` recording its
byte offset and length within the source string. The operation is infallible — any input
string SHALL produce a valid sequence; malformed structures degrade gracefully without
raising errors.

> test: code

### Scenario: Empty input produces no elements

- **WHEN** the source string is empty
- **THEN** the resulting element sequence is empty

### Scenario: Mixed content produces an ordered sequence of distinct kinds

- **GIVEN** a source containing a heading, a paragraph, list items, a block quote, and a
  trailing paragraph

- **WHEN** the source is parsed

- **THEN** the elements appear in source order

- **AND** each construct produces its corresponding `Element` variant

### Scenario: Element spans match byte offsets in the source

- **GIVEN** a source with a heading followed by a blank line and a paragraph
- **WHEN** the source is parsed
- **THEN** each element's `span.offset` equals its starting byte offset in the source
- **AND** `span.length` covers the element's bytes including its trailing newline

### Scenario: Unclosed code block is flushed at end of input

- **GIVEN** a source containing an opening fence with no matching closing fence

- **WHEN** the source is parsed

- **THEN** a single `Block(CodeBlock)` is emitted with content collected from the opening
  fence through end of input

## Requirement: ATX heading classification

A line beginning with one to six `#` characters followed by a single space and non-empty
content SHALL produce a `Heading` element with the corresponding level and the trailing
content as text. Lines beginning with `#` followed by no space, or with more than six
leading `#` characters, SHALL NOT produce a `Heading` and instead fall through to the
paragraph aggregator. A heading line SHALL terminate any pending paragraph or list item.

> test: code

### Scenario: ATX headings at levels 1 through 6

- **GIVEN** a source with `# H1`, `## H2`, `### H3`, `#### H4`, `##### H5`, and
  `###### H6` on consecutive lines

- **WHEN** the source is parsed

- **THEN** six `Heading` elements are produced with levels 1 through 6

- **AND** each element's content is the heading text without the leading hashes

### Scenario: Hashes without a following space become paragraphs

- **GIVEN** a source line `#hashtag`
- **WHEN** the source is parsed
- **THEN** the line becomes a `Block(Paragraph)`, not a `Heading`

### Scenario: A heading terminates the preceding paragraph

- **GIVEN** a source containing a paragraph immediately followed by an H2
- **WHEN** the source is parsed
- **THEN** a `Block(Paragraph)` element is emitted before the `Heading`
- **AND** both elements appear in the resulting sequence

## Requirement: Paragraph aggregation

Consecutive non-classified lines SHALL be aggregated into a single `Block(Paragraph)`
element. A blank line, a heading line, a list item line, a block-quote line, or a code
fence line SHALL terminate the current paragraph.

> test: code

### Scenario: Multi-line paragraph stays a single block

- **GIVEN** a source with three consecutive non-blank, non-classified lines
- **WHEN** the source is parsed
- **THEN** a single `Block(Paragraph)` element is produced
- **AND** its content is the three source lines joined by newlines

### Scenario: Blank line separates paragraphs

- **GIVEN** two paragraphs separated by a single blank line
- **WHEN** the source is parsed
- **THEN** two distinct `Block(Paragraph)` elements are produced

## Requirement: Fenced code blocks

A line whose trimmed content begins with three backticks SHALL open a fenced code block.
The block continues until a line whose trimmed content is exactly three backticks (or
three backticks followed by a space). The opening fence, the closing fence, any info
string after the opening fence, and any blank lines within the block SHALL be preserved
verbatim in the resulting `Block(CodeBlock)` content.

> test: code

### Scenario: Code block preserves content verbatim

- **GIVEN** a source `\`\`\`rust\nfn main() {}\n\`\`\``
- **WHEN** the source is parsed
- **THEN** one `Block(CodeBlock)` element is produced
- **AND** its content is the input including both fences and the info string

### Scenario: Code block preserves blank lines and info strings

- **GIVEN** a fenced code block with an info string and an internal blank line
- **WHEN** the source is parsed
- **THEN** one `Block(CodeBlock)` element is produced
- **AND** the info string and the internal blank line appear in its content

## Requirement: List item recognition

A line beginning with `- ` (bullet) or with one or more digits followed by `. ` (numbered)
SHALL produce a `ListItem` element. The element records the marker variant (`Bullet` or
`Numbered`); the original numeric value of a numbered marker is discarded. The
leading-space count before the marker SHALL be recorded as the item's `indent`. A
continuation line whose leading whitespace reaches or exceeds the marker's content column
SHALL be appended to the current `ListItem`. A blank line, a new list item at any indent,
a heading, a block quote, or a code fence SHALL terminate the current list item. Lines
that resemble but do not match the list-item form (e.g. digit-dot with no following space,
version numbers) SHALL fall through to paragraph aggregation.

> test: code

### Scenario: Bullet and numbered markers are distinguished

- **GIVEN** a source containing both a bullet item `- foo` and a numbered item `1. bar`
- **WHEN** the source is parsed
- **THEN** one `ListItem` is produced with `Bullet` marker
- **AND** one `ListItem` is produced with `Numbered` marker

### Scenario: Indent records nesting level

- **GIVEN** a source `- Outer` followed by `  - Inner`
- **WHEN** the source is parsed
- **THEN** two `ListItem` elements are produced
- **AND** the first has `indent` 0
- **AND** the second has `indent` 2

### Scenario: Continuation lines aligned with content are absorbed

- **GIVEN** a source containing a list item followed by a continuation line indented to
  the marker's content column

- **WHEN** the source is parsed

- **THEN** a single `ListItem` is produced

- **AND** its content includes both the first line and the continuation

### Scenario: Loose lists produce one ListItem per item

- **GIVEN** three list items separated by blank lines
- **WHEN** the source is parsed
- **THEN** three distinct `ListItem` elements are produced

### Scenario: Double-digit numbered markers are recognized

- **GIVEN** a source `10. Tenth` followed by `11. Eleventh`
- **WHEN** the source is parsed
- **THEN** two `ListItem` elements with `Numbered` marker are produced

### Scenario: List look-alikes fall back to paragraphs

- **GIVEN** a source line such as `1.no space` or `1.0 version`
- **WHEN** the source is parsed
- **THEN** the line becomes a `Block(Paragraph)`, not a `ListItem`

## Requirement: Block quote recognition

A line beginning with `> ` or consisting solely of `>` SHALL produce a `BlockQuoteItem`
element carrying the line's content with the `>` marker stripped. Each block-quote line
SHALL produce its own element — block quotes are not aggregated into a multi-line element.
A block-quote line SHALL terminate any pending paragraph or list item.

> test: code

### Scenario: Each block quote line produces its own element

- **GIVEN** a source with two consecutive block-quote lines
- **WHEN** the source is parsed
- **THEN** two distinct `BlockQuoteItem` elements are produced

### Scenario: Block-quote-formatted list-like content stays a block quote

- **GIVEN** a source with a block-quote line followed by a block-quote line whose content
  begins with `- `

- **WHEN** the source is parsed

- **THEN** two `BlockQuoteItem` elements are produced

- **AND** no `ListItem` is produced

### Scenario: A block quote terminates a list item

- **GIVEN** a list item line immediately followed by a block-quote line
- **WHEN** the source is parsed
- **THEN** a `ListItem` is produced followed by a `BlockQuoteItem`
