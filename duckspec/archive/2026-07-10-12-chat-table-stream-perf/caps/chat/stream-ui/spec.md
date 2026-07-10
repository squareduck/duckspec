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

### Scenario: Reasoning deltas accumulate on the session without materialization

- **GIVEN** a streaming turn whose chat UI has not been re-materialized since the last
  materialization

- **WHEN** one or more reasoning content deltas are applied

- **THEN** the session’s live reasoning buffer includes every delta’s text in order

- **AND** that text is present on the session whether or not chat UI materialization has
  run for those deltas

> test: code

## Requirement: Bounded materialization while streaming

While a turn is streaming, pure answer or reasoning content deltas alone SHALL NOT each
force chat UI materialization. Accumulated pure-content dirtiness SHALL materialize on the
stream UI tick only while the transcript is stick-to-bottom (the user is following the
live answer). While the user has scrolled up to read history, pure-content dirtiness SHALL
remain deferred so the chat column is not rebuilt under their scroll; re-engaging
stick-to-bottom SHALL materialize any deferred pure content. Structural transcript changes
— tool use, tool result, a kind switch that flushes the other pending buffer, turn
complete, error, or process exit — SHALL materialize the chat UI as part of handling that
event, without waiting for a stream UI tick and regardless of stick-to-bottom.

> test: code

### Scenario: Pure content deltas alone do not materialize the chat UI

- **GIVEN** a streaming turn

- **AND** only answer or reasoning content deltas have been applied since the last chat UI
  materialization

- **WHEN** those deltas are handled without a stream UI tick and without a structural
  transcript event

- **THEN** chat UI materialization does not run for those deltas alone

> test: code

### Scenario: Stream UI tick materializes accumulated session answer text into the chat UI

- **GIVEN** a streaming turn whose session holds answer text not yet reflected in the chat
  UI

- **AND** the transcript is stick-to-bottom

- **WHEN** the stream UI tick fires

- **THEN** chat UI materialization runs

- **AND** the live answer presented by the chat UI includes that session answer text

> test: code

### Scenario: Stream UI tick skips materialize while scrolled up in history

- **GIVEN** a streaming turn with pure-content dirtiness on the session
- **AND** the transcript is not stick-to-bottom
- **WHEN** the stream UI tick fires
- **THEN** chat UI materialization does not run
- **AND** the session still holds the accumulated pure-content text

> test: code

### Scenario: Re-sticking to bottom materializes deferred content

- **GIVEN** a streaming turn with pure-content dirtiness deferred while not
  stick-to-bottom

- **WHEN** the transcript becomes stick-to-bottom again

- **THEN** chat UI materialization runs

- **AND** the live answer presented by the chat UI includes the deferred session text

> test: code

### Scenario: Tool use materializes the chat UI immediately with an Activity row

- **GIVEN** a streaming turn
- **WHEN** a tool use is applied to the session
- **THEN** chat UI materialization runs as part of handling that event
- **AND** the chat UI includes an Activity row for that tool

> test: code

### Scenario: Turn complete materializes the final answer immediately

- **GIVEN** a streaming turn with answer text on the session

- **WHEN** the turn completes

- **THEN** chat UI materialization runs as part of handling that event

- **AND** the chat UI presents the final answer text without waiting for a further stream
  UI tick

> test: code

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

### Scenario: Suffix-growing live answer refreshes in place

- **GIVEN** a live answer block whose new lines share a prefix with the previous
  materialization and only append or extend the suffix

- **WHEN** chat UI materialization runs

- **THEN** that block’s editor is refreshed in place

- **AND** the editor is not constructed as a brand-new editor from the full joined text

> test: code

### Scenario: Block list reshape uses full rebuild for affected indices

- **GIVEN** a materialization in which a block index changes kind or appears as a new
  segment (for example a tool Activity inserted before the live answer)

- **WHEN** chat UI materialization runs

- **THEN** each affected index receives a full editor rebuild appropriate to its new
  content

- **AND** any earlier block whose lines are unchanged keeps its existing editor

> test: code

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

### Scenario: Cache hit shares layout geometry without deep-cloning the tree

- **GIVEN** a hybrid layout cache entry for a key
- **WHEN** hybrid layout is requested with that key
- **THEN** the consumer receives shared access to the cached geometry
- **AND** satisfying the request does not require deep-cloning the full layout tree

> test: code
