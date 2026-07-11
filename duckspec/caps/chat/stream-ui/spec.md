# Chat stream UI

While a turn streams, session text always applies immediately, but the chat UI
materializes on a bounded cadence (plus structural immediacy). Settled blocks keep their
editors; hybrid table layout for unchanged inputs is reused without recompute or deep copy
of the layout tree.

## Requirement: Session apply before materialize

Stream events that carry answer or reasoning text SHALL update the session’s live buffers
(or committed messages after a flush) even when chat UI materialization is deferred.
Deferred materialization SHALL NOT drop stream text from the session.

> test: code

### Scenario: Content deltas accumulate on the session without materialization

- **GIVEN** a streaming turn whose chat UI has not been re-materialized since the last
  materialization

- **WHEN** one or more answer content deltas are applied

- **THEN** the session’s live answer buffer (or equivalent pending answer text) includes
  every delta’s text in order

- **AND** that text is present on the session whether or not chat UI materialization has
  run for those deltas

> test: code
> - crates/duckboard/src/area/interaction.rs:1072

### Scenario: Reasoning deltas accumulate on the session without materialization

- **GIVEN** a streaming turn whose chat UI has not been re-materialized since the last
  materialization

- **WHEN** one or more reasoning content deltas are applied

- **THEN** the session’s live reasoning buffer includes every delta’s text in order

- **AND** that text is present on the session whether or not chat UI materialization has
  run for those deltas

> test: code
> - crates/duckboard/src/area/interaction.rs:1099

## Requirement: Bounded materialization while streaming

While a turn is streaming, pure answer or reasoning content deltas alone SHALL NOT each
force chat UI materialization. Accumulated pure-content dirtiness SHALL materialize on the
stream UI tick only while the transcript is stick-to-bottom (the user is following the
live answer). While the user has scrolled up to read history, pure-content dirtiness SHALL
remain deferred so the chat column is not rebuilt under their scroll; re-engaging
stick-to-bottom SHALL materialize any deferred pure content. Structural transcript changes
— tool use, tool result, a kind switch between answer and reasoning channels (whether or
not the open answer draft is committed), turn complete, error, or process exit — SHALL
materialize the chat UI as part of handling that event, without waiting for a stream UI
tick and regardless of stick-to-bottom.

> test: code

### Scenario: Pure content deltas alone do not materialize the chat UI

- **GIVEN** a streaming turn

- **AND** only answer or reasoning content deltas have been applied since the last chat UI
  materialization

- **WHEN** those deltas are handled without a stream UI tick and without a structural
  transcript event

- **THEN** chat UI materialization does not run for those deltas alone

> test: code
> - crates/duckboard/src/area/interaction.rs:1125

### Scenario: Stream UI tick materializes accumulated session answer text into the chat UI

- **GIVEN** a streaming turn whose session holds answer text not yet reflected in the chat
  UI

- **AND** the transcript is stick-to-bottom

- **WHEN** the stream UI tick fires

- **THEN** chat UI materialization runs

- **AND** the live answer presented by the chat UI includes that session answer text

> test: code
> - crates/duckboard/src/area/interaction.rs:1161

### Scenario: Stream UI tick skips materialize while scrolled up in history

- **GIVEN** a streaming turn with pure-content dirtiness on the session
- **AND** the transcript is not stick-to-bottom
- **WHEN** the stream UI tick fires
- **THEN** chat UI materialization does not run
- **AND** the session still holds the accumulated pure-content text

> test: code
> - crates/duckboard/src/area/interaction.rs:1194

### Scenario: Re-sticking to bottom materializes deferred content

- **GIVEN** a streaming turn with pure-content dirtiness deferred while not
  stick-to-bottom

- **WHEN** the transcript becomes stick-to-bottom again

- **THEN** chat UI materialization runs

- **AND** the live answer presented by the chat UI includes the deferred session text

> test: code
> - crates/duckboard/src/area/interaction.rs:1212

### Scenario: Tool use materializes the chat UI immediately with an Activity row

- **GIVEN** a streaming turn
- **WHEN** a tool use is applied to the session
- **THEN** chat UI materialization runs as part of handling that event
- **AND** the chat UI includes an Activity row for that tool

> test: code
> - crates/duckboard/src/area/interaction.rs:1241

### Scenario: Turn complete materializes the final answer immediately

- **GIVEN** a streaming turn with answer text on the session

- **WHEN** the turn completes

- **THEN** chat UI materialization runs as part of handling that event

- **AND** the chat UI presents the final answer text without waiting for a further stream
  UI tick

> test: code
> - crates/duckboard/src/area/interaction.rs:1283

### Scenario: Answer-to-reasoning channel switch materializes without committing the answer

- **GIVEN** a streaming turn with a non-empty live answer draft
- **WHEN** a reasoning content delta is applied (answer channel to reasoning channel)
- **THEN** chat UI materialization runs as part of handling that event
- **AND** the open answer draft remains uncommitted on the session

> test: code
> - crates/duckboard/src/area/interaction.rs:1403

## Requirement: Settled and live editor refresh

On chat UI materialization, a block whose lines are unchanged from the previous
materialization SHALL keep its existing editor (no full replace). When the live open
answer or thinking block only grows by a suffix append — a shared line prefix, optional
growth of the last shared line, then zero or more new lines — materialization SHALL
refresh that editor in place rather than constructing a brand-new editor from the full
joined text. When the block list reshapes (new block indices, kind changes, or non-suffix
content edits), affected indices MAY use a full editor rebuild; unchanged earlier blocks
still keep their editors.

> test: code

### Scenario: Unchanged settled block keeps its editor across materialize

- **GIVEN** a chat UI whose earlier block lines match the next materialization for that
  index

- **AND** a later live block whose content has grown

- **WHEN** chat UI materialization runs

- **THEN** the earlier block continues to use its existing editor instance

- **AND** that editor is not replaced by a newly constructed editor for the same lines

> test: code
> - crates/duckboard/src/area/interaction.rs:1533

### Scenario: Suffix-growing live answer refreshes in place

- **GIVEN** a live answer block whose new lines share a prefix with the previous
  materialization and only append or extend the suffix

- **WHEN** chat UI materialization runs

- **THEN** that block’s editor is refreshed in place

- **AND** the editor is not constructed as a brand-new editor from the full joined text

> test: code
> - crates/duckboard/src/area/interaction.rs:1573

### Scenario: Block list reshape uses full rebuild for affected indices

- **GIVEN** a materialization in which a block index changes kind or appears as a new
  segment (for example a tool Activity inserted before the live answer)

- **WHEN** chat UI materialization runs

- **THEN** each affected index receives a full editor rebuild appropriate to its new
  content

- **AND** any earlier block whose lines are unchanged keeps its existing editor

> test: code
> - crates/duckboard/src/area/interaction.rs:1613

## Requirement: Hybrid layout reuse

When hybrid layout is requested twice with the same pane width, wrap setting, and
line-buffer identity and version, the second request SHALL reuse the previously computed
layout geometry without re-running table layout over the line buffer. A cache hit SHALL
share that geometry so consumers can read it without a deep copy of the full layout tree.

> test: code

### Scenario: Second hybrid layout request with the same key does not recompute tables

- **GIVEN** a hybrid layout already computed for a line buffer, pane width, wrap setting,
  and buffer version

- **WHEN** hybrid layout is requested again with the same key

- **THEN** table layout is not re-run over the line buffer

- **AND** the returned geometry matches the previously computed layout

> test: code
> - crates/duckboard/src/widget/text_edit/render.rs:3155

### Scenario: Cache hit shares layout geometry without deep-cloning the tree

- **GIVEN** a hybrid layout cache entry for a key
- **WHEN** hybrid layout is requested with that key
- **THEN** the consumer receives shared access to the cached geometry
- **AND** satisfying the request does not require deep-cloning the full layout tree

> test: code
> - crates/duckboard/src/widget/text_edit/render.rs:3179

## Requirement: Answer draft across thought

While a turn is streaming, a reasoning content delta SHALL NOT commit the open answer
draft into the session’s messages. When answer content resumes after reasoning while an
answer draft is already open, the session SHALL replace that draft with the new answer
content (the prior draft text is discarded). Applying a tool use SHALL commit the open
answer draft into the session’s messages before the tool is recorded.

> test: code

### Scenario: Reasoning leaves the open answer uncommitted

- **GIVEN** a streaming turn with a non-empty live answer draft

- **WHEN** a reasoning content delta is applied

- **THEN** the session still holds that answer text only as the live answer draft

- **AND** the session’s committed messages do not gain a new answer text block for that
  draft

> test: code
> - crates/duckboard/src/area/interaction.rs:1331

### Scenario: Answer after reasoning replaces the live draft

- **GIVEN** a streaming turn whose live answer draft is a known first body
- **AND** a reasoning content delta has been applied after that draft
- **WHEN** an answer content delta with a different second body is applied
- **THEN** the live answer draft is the second body
- **AND** the live answer draft does not retain the first body

> test: code
> - crates/duckboard/src/area/interaction.rs:1349

### Scenario: Tool use commits the open answer draft

- **GIVEN** a streaming turn with a non-empty live answer draft
- **WHEN** a tool use is applied to the session
- **THEN** the session’s committed messages include that answer text
- **AND** the live answer draft is empty

> test: code
> - crates/duckboard/src/area/interaction.rs:1377

## Requirement: Answer thrash budget

Within one streaming turn, after two answer-after-thought draft replacements, a third
answer-after-thought replacement SHALL cancel the in-flight turn, keep the last live
answer draft as the turn’s answer, and surface a short stop notice that is not an answer
rewrite. The replacement count SHALL reset when a tool use is applied so answer spans
separated by tools do not share a budget.

> test: code

### Scenario: Third answer-after-thought cancels and keeps the last draft

- **GIVEN** a streaming turn that has already replaced the live answer draft twice after
  reasoning (two answer-after-thought replacements)

- **WHEN** a third answer-after-thought replacement begins (answer content after reasoning
  with a non-empty draft)

- **THEN** the in-flight turn is cancelled

- **AND** the session keeps the last live answer draft as the turn’s answer

- **AND** a short stop notice is shown that is not a second full answer rewrite

> test: code
> - crates/duckboard/src/area/interaction.rs:1434

### Scenario: Tool use resets the thrash budget

- **GIVEN** a streaming turn that has already performed two answer-after-thought draft
  replacements

- **AND** a tool use has since been applied (budget reset)

- **WHEN** answer content is applied after further reasoning with a non-empty draft
  (another answer-after-thought replacement)

- **THEN** the in-flight turn is not cancelled solely for exceeding the thrash budget

> test: code
> - crates/duckboard/src/area/interaction.rs:1475
