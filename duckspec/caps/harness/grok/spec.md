# Grok harness

The grok harness drives the grok CLI over ACP, starting or resuming a session per turn and
translating grok's `session/update` stream into duckchat's neutral agent events, including
accurate context usage.

## Requirement: Session lifecycle and resume

Running a turn without a prior session id SHALL open a new grok session and report the id
grok assigns. Running a turn with a prior session id SHALL open by resuming that id. In
both cases the reported session id SHALL be surfaced for the caller to persist.

> test: code

### Scenario: A turn without a prior session opens a new session

- **GIVEN** a turn request carrying no session id
- **WHEN** the harness runs the turn
- **THEN** it opens a fresh grok session
- **AND** it surfaces the session id grok assigned

> test: code
> - crates/duckchat/src/grok/acp.rs:445

### Scenario: A turn with a prior session id resumes that session

- **GIVEN** a turn request carrying a previously-assigned session id
- **WHEN** the harness runs the turn
- **THEN** it opens the session by resuming that same id

> test: code
> - crates/duckchat/src/grok/acp.rs:485

## Requirement: Event translation

The harness SHALL translate grok's session updates into neutral agent events: assistant
text and reasoning SHALL surface on separate channels; a tool invocation SHALL surface as
a tool-use event followed by a result event sharing the same call id; and token telemetry
SHALL surface as a usage update carrying the used-token count together with the active
model's context window.

> test: code

### Scenario: Assistant text and reasoning surface on distinct channels

- **GIVEN** a session update stream containing both an assistant message chunk and a
  reasoning chunk

- **WHEN** the harness translates the stream

- **THEN** the assistant text is emitted as a content event

- **AND** the reasoning text is emitted as a separate reasoning event

> test: code
> - crates/duckchat/src/grok/event.rs:123

### Scenario: A tool call surfaces as a use then a matching result

- **GIVEN** a session update stream containing a tool call and its completion
- **WHEN** the harness translates the stream
- **THEN** a tool-use event is emitted with the call's id, name, and input
- **AND** a tool-result event is emitted carrying the same call id and the tool output

> test: code
> - crates/duckchat/src/grok/event.rs:153

### Scenario: A usage update carries used tokens and the model's context window

- **GIVEN** a session update reporting a running total-token count
- **AND** an active model whose context window is known
- **WHEN** the harness translates the update
- **THEN** a usage event is emitted with that used-token count and that context window

> test: code
> - crates/duckchat/src/grok/event.rs:196

## Requirement: Model discovery

Listing models SHALL return grok's available models, each tagged with the grok harness and
carrying its own context window. The title summariser SHALL select the cheapest available
model and SHALL fall back to another available model when the preferred fast model is
absent.

> test: code

### Scenario: Discovered models are tagged with the grok harness and a window

- **GIVEN** a grok handshake advertising its available models
- **WHEN** the harness lists models
- **THEN** each returned model is tagged with the grok harness
- **AND** each returned model carries a context window

> test: code
> - crates/duckchat/src/grok.rs:410

### Scenario: Title model falls back when the preferred fast model is absent

- **GIVEN** a set of available models that does not include the preferred fast model
- **WHEN** the harness selects a model for title summarisation
- **THEN** it selects another available model rather than failing

> test: code
> - crates/duckchat/src/grok.rs:428

## Requirement: Graceful unavailability

When the grok binary or its authentication is unavailable, listing models SHALL return an
empty list and running a turn SHALL fail with a typed error rather than panicking.

> test: code

### Scenario: A missing grok binary yields no models and a turn error

- **GIVEN** an environment where the grok binary cannot be launched
- **WHEN** the harness lists models and then runs a turn
- **THEN** the model list is empty
- **AND** the turn fails with a typed error rather than panicking

> test: code
> - crates/duckchat/src/grok.rs:443

## Requirement: Prompt attachments

When assembling a turn for `session/prompt`, the harness SHALL walk the folded prompt text
for markdown links of the form `[label](attach:<id>)`, resolve each link against the
turn's attachments map, and send a multi-block ACP `prompt` array. A resolved image
attachment SHALL appear as an ACP image content block carrying that attachment's media
type and payload. Surrounding text SHALL appear as text content blocks. A resolved
non-image attachment SHALL appear as a text content block rather than an image block. An
unresolved `attach:` link SHALL be left as its original literal markdown text.

> test: code

### Scenario: A resolved image attachment is sent as an ACP image block

- **GIVEN** a turn whose prompt contains an `attach:` link
- **AND** the turn's attachments map holds an image payload for that link's id
- **WHEN** the harness assembles the prompt for the turn
- **THEN** the `session/prompt` content includes an image content block
- **AND** that block carries the attachment's media type and payload

> test: code
> - crates/duckchat/src/grok.rs:343

### Scenario: Surrounding text is preserved as text blocks

- **GIVEN** a prompt with text before and after a resolved image `attach:` marker

- **WHEN** the harness assembles the prompt for the turn

- **THEN** the text before the marker appears as a text content block before the image
  block

- **AND** the text after the marker appears as a text content block after the image block

> test: code
> - crates/duckchat/src/grok.rs:363

### Scenario: A non-image attachment is represented as text

- **GIVEN** a turn whose prompt contains an `attach:` link
- **AND** the turn's attachments map holds a non-image payload for that link's id
- **WHEN** the harness assembles the prompt for the turn
- **THEN** the attachment is represented as a text content block
- **AND** the content does not include an image content block for that attachment

> test: code
> - crates/duckchat/src/grok.rs:379

### Scenario: An unresolved attach marker is left literal

- **GIVEN** a turn whose prompt contains an `attach:` link
- **AND** the turn's attachments map has no entry for that link's id
- **WHEN** the harness assembles the prompt for the turn
- **THEN** the original markdown link remains as text content

> test: code
> - crates/duckchat/src/grok.rs:401
