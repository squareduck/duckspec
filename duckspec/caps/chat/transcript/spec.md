# Chat transcript

Build a harness-neutral transcript of Thinking, Activity, and Answer segments from stored
content and live stream buffers — with id-paired tool groups and settled collapse defaults
so the answer stays primary.

## Requirement: Segment construction

Contiguous same-kind assistant content SHALL coalesce into one Thinking, Activity, or
Answer segment; a kind switch SHALL open a new segment. While streaming, pending reasoning
and pending answer text SHALL appear on live Thinking / Answer segments rather than as
separate committed messages until flushed. When both pending reasoning and pending answer
text are open, the transcript SHALL present one live Thinking segment and one live Answer
segment (not multiple Answer segments for the same uncommitted draft).

> test: code

### Scenario: Reasoning then answer yields Thinking then Answer

- **GIVEN** a session whose assistant content is a reasoning block followed by a text
  block

- **WHEN** the transcript segments are built

- **THEN** the segments are a Thinking segment then an Answer segment

- **AND** the reasoning body is not part of the Answer segment

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2436

### Scenario: Contiguous tools yield one Activity with multiple rows

- **GIVEN** a session whose assistant content is several consecutive tool uses and their
  results

- **WHEN** the transcript segments are built

- **THEN** those tools form a single Activity segment

- **AND** the segment has one row per tool call

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2469

### Scenario: Thought, tools, thought, answer yields four segments in order

- **GIVEN** a session whose assistant content is reasoning, then tools, then reasoning,
  then text

- **WHEN** the transcript segments are built

- **THEN** the segments are Thinking, Activity, Thinking, Answer in that order

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2496

### Scenario: Live pending reasoning appears on an open Thinking segment

- **GIVEN** a streaming session with non-empty pending reasoning and no committed
  reasoning for that run yet

- **WHEN** the transcript segments are built

- **THEN** a live Thinking segment includes that pending reasoning text

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2520

### Scenario: Live reasoning with an open answer draft yields Thinking then one Answer

- **GIVEN** a streaming session with non-empty pending reasoning and non-empty pending
  answer text

- **WHEN** the transcript segments are built

- **THEN** the live segments include a Thinking segment then an Answer segment

- **AND** there is exactly one Answer segment for that open draft

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2543

## Requirement: Activity pairing

Within an Activity segment, tool uses and results SHALL pair by call id (not adjacency
alone). A completed row SHALL carry the tool summary and result body. A result whose use
is missing SHALL still form a done row labeled from the result's tool name — never a
generic "done" placeholder alone.

> test: code

### Scenario: Matching use and result become one done row

- **GIVEN** a tool use and a tool result that share the same call id
- **WHEN** the transcript segments are built
- **THEN** the Activity segment has one done row for that id
- **AND** the row carries the tool summary and the result body

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2579

### Scenario: Non-adjacent use and result still pair by id

- **GIVEN** two tool uses and two results ordered so each result is not immediately after
  its matching use

- **AND** each result shares a call id with exactly one of the uses

- **WHEN** the transcript segments are built

- **THEN** each use is paired with its matching result into one done row

- **AND** no row is labeled only as a generic done placeholder

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2602

### Scenario: Orphan result is a named done row

- **GIVEN** a tool result with no preceding tool use for the same call id
- **WHEN** the transcript segments are built
- **THEN** the Activity segment includes a done row labeled from the result's tool name
- **AND** the row is not labeled only as a generic done placeholder

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2634

## Requirement: Collapse defaults

Live Thinking and live Activity segments SHALL start expanded. A Thinking segment SHALL
auto-collapse when a following Answer segment appears or the turn completes, unless the
user has toggled that segment. An Activity segment SHALL auto-collapse when the turn
settles (following Answer or turn complete), unless the user has toggled it. On reload of
a finished turn, Thinking and Activity SHALL start collapsed.

> test: code

### Scenario: Thinking collapses when answer follows

- **GIVEN** a live Thinking segment that the user has not toggled
- **WHEN** a following Answer segment appears for the same turn
- **THEN** the Thinking segment is collapsed

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:3028

### Scenario: User-expanded Thinking is not auto-collapsed

- **GIVEN** a Thinking segment the user has expanded
- **WHEN** a following Answer segment appears for the same turn
- **THEN** the Thinking segment remains expanded

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:3085

### Scenario: Settled Activity starts collapsed

- **GIVEN** a finished turn whose transcript includes an Activity segment
- **WHEN** the transcript is presented for that settled turn
- **THEN** the Activity segment is collapsed

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:3125

## Requirement: Segment presentation

Collapsed Thinking SHALL label by line count (no duration). Collapsed Activity SHALL
summarize as a count plus sample tool names. Expanded Activity SHALL show one quiet row
per tool (status + summary) with truncated output under the row when present — group
expand only, with no nested per-tool expand state.

> test: code

### Scenario: Thinking collapsed label includes line count

- **GIVEN** a Thinking segment whose body has a known number of lines
- **WHEN** the collapsed label for that segment is produced
- **THEN** the label includes that line count
- **AND** the label does not include a duration

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2656

### Scenario: Activity collapsed label includes count and sample names

- **GIVEN** an Activity segment with multiple completed tools
- **WHEN** the collapsed label for that segment is produced
- **THEN** the label includes the tool count
- **AND** the label includes sample tool names from the rows

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2689

### Scenario: Expanded activity exposes status, summary, and truncated output

- **GIVEN** an expanded Activity segment with a completed tool that produced multi-line
  output

- **WHEN** the segment's rows are presented

- **THEN** each tool has one row showing its status and summary

- **AND** truncated output is available under the row for that tool

- **AND** no separate per-tool expand state is required to show that truncated output

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2721

## Requirement: Meta-card line background

When an Answer segment's text is prepared for display, every line whose index falls in a
meta-card inclusive line range for that answer text SHALL receive a meta-card line
background. Lines outside those ranges SHALL NOT receive a meta-card line background.
Meta-card ranges are those produced by chat meta-card recognition for `write` and `next`
cards in the answer source. The background SHALL be visually distinct from ordinary Answer
text and from search-match and diff line backgrounds.

> test: code

### Scenario: Meta-card lines on an Answer get meta-card background

- **GIVEN** an Answer whose source text contains a recognized `next` meta card covering a
  known inclusive line range

- **WHEN** that Answer's display lines are prepared

- **THEN** every line index in that range has a meta-card line background

> test: code
> - crates/duckboard/src/meta_card.rs:368

### Scenario: Non-meta lines on the same Answer do not get meta-card background

- **GIVEN** an Answer whose source text has ordinary prose lines before a recognized meta
  card

- **WHEN** that Answer's display lines are prepared

- **THEN** the ordinary prose lines do not have a meta-card line background

> test: code
> - crates/duckboard/src/meta_card.rs:392

## Requirement: Thinking body fade

Expanded Thinking body text SHALL use a text color that is more faded than Answer body
text in the same theme, while remaining legible. Thinking headers MAY use a more muted
color than the Thinking body.

### Scenario: Thinking body is more faded than Answer body

- **GIVEN** a transcript with an expanded Thinking segment and an Answer segment
- **WHEN** both bodies are presented in the chat UI
- **THEN** the Thinking body appears more faded than the Answer body
- **AND** the Thinking body remains legible

> manual: visual contrast in light and dark
