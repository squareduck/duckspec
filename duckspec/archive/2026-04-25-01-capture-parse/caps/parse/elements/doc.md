# Markdown elements

Layer 1 of the parsing pipeline. Tokenizes a markdown source string into a flat sequence
of `Element` values (Heading, Block, ListItem, BlockQuoteItem) with byte-offset spans.

## Layer 1 vs Layer 2

The parsing pipeline has two layers with a clean boundary:

```
  raw markdown source
         │
         ▼
  ┌────────────────────┐
  │  parse_elements    │   Layer 1 — line-by-line state machine
  │   (infallible)     │   produces Vec<Element>
  └────────────────────┘
         │
         ▼
  ┌────────────────────┐
  │ parse_spec / doc / │   Layer 2 — artifact-shape validation
  │  delta / step      │   produces typed artifacts or
  │  (fallible)        │   Vec<ParseError>
  └────────────────────┘
```

Layer 1 owns markdown mechanics; Layer 2 owns artifact shape. Layer 2 parsers never
re-tokenize markdown — they consume the `Vec<Element>` produced by Layer 1 and validate
structure on top of it.

## Element variants

The `Element` enum has four variants. Every variant carries a `Span`.

```
| Variant         | Produced by                          | Carries                          |
|-----------------|--------------------------------------|----------------------------------|
| Heading         | ATX `# text` lines (levels 1–6)      | level, content text              |
| Block           | paragraphs and fenced code blocks    | content, kind (Paragraph/Code)   |
| ListItem        | `- text` and `N. text` lines         | content, indent, marker          |
| BlockQuoteItem  | `> text` and `>` lines               | content                          |
```

Headings carry the heading text without leading hashes. Code blocks preserve the opening
and closing fences, any info string, and internal blank lines verbatim. List items record
their marker variant (`Bullet` or `Numbered`); the original numeric value of a numbered
marker is dropped at parse time and the renderer renumbers from 1 within each run.
Block-quote items are always single-line; multi-line block quotes are represented as a
sequence of `BlockQuoteItem` elements.

## Spans

Every element carries a `Span { offset, length }` recording its byte range within the
source string. Spans are byte-indexed (not character- or line-indexed). The `Span` struct
is `Copy` and converts directly to `miette::SourceSpan` for diagnostic rendering.

```
Source: "# Title\n\nBody text"
                                       offset  length
        ─────────────────────────────  ──────  ──────
        # Title (Heading, level 1)          0       8   ← includes trailing \n
        Body text (Paragraph)               9       9   ← no trailing \n at EOF
```

Layer 2 parsers attach the spans of the elements they consume to the artifacts they
produce, so error messages and downstream tooling can pinpoint source locations.

## State machine

Layer 1 is a line-by-line state machine. Each line either continues the current state or
flushes it and starts a new one.

```
                  blank line / classified line
            ┌──────────────────────────────────────┐
            │                                      │
            ▼                                      │
  ┌─────────────────┐  text line                   │
  │     Normal      │ ─────────────────────────────┤
  └─────────────────┘                              │
       │  ▲                                        │
       │  │ flush                                  ▼
       │  │                            ┌─────────────────┐
       │  └─── close fence ─────────── │   InCodeBlock   │
       │                               └─────────────────┘
       │  open fence
       │
       │  list-item start
       ▼
  ┌─────────────────┐ continuation       ┌─────────────────┐
  │   InListItem    │ ─────────────────→ │  InListItem     │
  └─────────────────┘                    └─────────────────┘
       │
       │ blank / new item / heading / quote / fence
       ▼
   flush, reprocess in Normal

  ┌─────────────────┐ continuation       ┌─────────────────┐
  │  InParagraph    │ ─────────────────→ │  InParagraph    │
  └─────────────────┘                    └─────────────────┘
       │
       │ blank / heading / list / quote / fence
       ▼
   flush, reprocess in Normal
```

The four states (`Normal`, `InParagraph`, `InCodeBlock`, `InListItem`) cover every line.
`InCodeBlock` only exits on a closing fence, which is what makes the contract "preserve
content verbatim" — fence-internal lines never trigger classification. End-of-input
flushes whatever state is pending, including unclosed code blocks.

## Infallibility contract

`parse_elements` is documented as infallible: any input string produces a valid
`Vec<Element>`. There is no `Result`, no panic, no error path. This is a load-bearing
guarantee for Layer 2 parsers, which assume their input is a well-formed element stream
and only validate artifact-level structure.

The contract degrades gracefully in three concrete ways:

- **Unclosed code blocks** flush at end of input as a `Block(CodeBlock)` with whatever
  content was collected.

- **List look-alikes** that don't match the strict marker form (e.g. `1.no-space`,
  `1.0 version`, `#hashtag`) fall back to paragraph aggregation rather than raising an
  error.

- **Lines that don't fit any classification** become paragraph content.

A future change that wanted to fail on, for example, unclosed fences would break every
Layer 2 parser that assumes its element stream is well-formed. That's why the
infallibility manifestations are pinned as `test: code` scenarios — they're not
nice-to-haves; they're the contract.
