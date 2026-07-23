# Style schema

Shared markdown style for **chat and on-disk artifacts**. One place for how GFM
should read - clean, consistent, shaped to the data - plus rare chat-only meta
cards. Not an artifact schema (no Structure/Rules/Quality/Formatting/Example
skeleton).

## Markdown

Same taste in a message and in a file. Form follows data:

| Situation | Prefer |
| --- | --- |
| Parallel items with shared fields (states, findings, scope rows, checks) | GFM table |
| Architecture, flow, state machine | ASCII diagram in a fenced block |
| Trees (caps, modules, files) | Indented tree in a fenced block |
| Single outcome or short path | Prose or a tight list |
| Paths and identifiers | Concrete `path` or `path:line` |
| Real code | Fenced block with a language tag |

**Craft:**

- Lead with structure when it helps (table or diagram before explanation)
- Depth is fine when it clarifies; skip ceremonial recap of known context
- Clean is not the same as brief - cut noise, not necessary detail
- In artifacts, put tables and diagrams in plain fenced blocks so formatters
  do not reflow them; language-tag only fences that hold real code
- Prefer one clear structure over mixed half-tables and half-prose for the same
  facts

## Diagrams

When a flow, state machine, or structure is easier to *see* than to read, draw
it. Prefer diagrams that look intentional, not hurried:

- Use box-drawing where it helps (`┌ ┐ └ ┘ │ ─ ► ▼`) and keep edges aligned
- Label edges with short conditions; label nodes with stable names from the prose
- One idea per diagram; split rather than overcrowding
- Symmetric spacing and columns scan better than freehand zigzags
- ASCII only (portable in plain fences); no decorative noise
- Follow with a short prose caption when the picture needs one beat of wording

Ugly `A --> B --> C` lines are a last resort when the graph is trivial. Prefer a
small, balanced figure when the relationship matters.

## Meta cards (chat only)

Blockquote cards are **chat chrome** for confirmations and choices. They never
appear in on-disk artifacts. Use them rarely and only when the user or client
must act. Findings, triage, verify results, and scopes stay ordinary markdown
(usually tables).

**Names:** always say **`write` meta card** and **`next` meta card** (kind in
backticks + the words “meta card”). Do not call them “next actions lists”,
“suggested next actions”, or bare “next” without “meta card” when instructing
the agent.

**Geometry** - every meta card:

```markdown
> **<kind>**
>
> <body>
```

- `<kind>` is one lowercase token from the closed set below
- A blank `>` line after the kind line is required
- Body is consecutive `>` lines; the card ends at the first non-blockquote line
- Only a blockquote whose first line is exactly `> **kind**` (known kind) counts
  as chrome; other quotes stay freeform

| Kind | Full name | Role |
| --- | --- | --- |
| `write` | `write` meta card | Intent: what will be written (not the full file) |
| `next` | `next` meta card | 1-3 ranked actions and/or await confirmation |

### `write` meta card

One or more short description lines inside the card (what path / artifact, plain
prose). The preview sits **outside** the card as normal markdown in the shape
you intend to write - real headings and structure, not a compact pseudo-syntax
line.

### `next` meta card

One to three body lines; list order is rank (first is primary):

```markdown
> **next**
>
> `<send>`
> `<send>`  <optional reason>
```

- Every send token is wrapped in backticks - slash commands and decision tokens
  alike: `/ds-step`, `confirm proposal`, `reject design`, `create change <name>`, …
- **Send tokens name the decision** (or the stage action). Bare `confirm` /
  `reject` are not the stock pattern - use decision-named forms such as
  `confirm proposal`, `confirm map`, `confirm <path>`
- **Reasons:** omit on decision tokens (the token is enough). Keep a short UI
  reason on slash-command handoffs only (a few words, e.g. `design the approach`)
- Column alignment of reasons is cosmetic only
- Omit the entire `next` meta card when there is nothing to offer

### Write gate

Compose **meta → information → meta** (`write` meta card, preview, `next` meta
card). The `write` meta card is a short plain description of the write. The
preview between cards is ordinary markdown in the shape of the artifact (or a
tight subset) - not a one-line pseudo-syntax.

```markdown
> **write**
>
> Proposal at `duckspec/changes/scope-aware-sessions/proposal.md`

# Scope-aware session orientation

Agents keep change context across turns via orientation payload.

The session begins with a compact orientation payload derived from the active
duckspec scope. It identifies the current change, progress, and likely next
stage without dumping project discovery into every conversation.

> **next**
>
> `confirm proposal`
> `reject proposal`
```

Always pair a `write` meta card with a `next` meta card that includes a
decision-named confirm token and usually a decision-named reject (or `revise`
when the alternative is edit-in-place).

### Handoff only

```markdown
> **next**
>
> `/ds-design`  design the approach
> `/ds-spec`    write specs
```

The agent chooses actions that fit the conversation. Templates say when to emit
a `next` meta card; they do not hardcode a disk-phase decision tree of stages.
