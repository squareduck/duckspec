# Grok harness

The grok harness drives the grok CLI over ACP, starting or resuming a session per turn and
translating grok's `session/update` stream into duckchat's neutral agent events, including
accurate context usage.

## Requirement: Session lifecycle and resume

Running a turn without a prior session id SHALL open a new grok session and report the id
grok assigns. Running a turn with a prior session id SHALL open by resuming that id. In
both cases the reported session id SHALL be surfaced for the caller to persist. When the
main path is already process-hot, a subsequent turn SHALL reuse that process rather than
spawning a new `grok agent stdio` child. Cancelling an in-flight turn SHALL kill the main
child; a later turn SHALL be allowed to spawn again and, when a prior session id is
supplied, resume that id.

> test: code

### Scenario: A turn without a prior session opens a new session

- **GIVEN** a turn request carrying no session id
- **WHEN** the harness runs the turn
- **THEN** it opens a fresh grok session
- **AND** it surfaces the session id grok assigned

> test: code
> - crates/duckchat/src/grok/acp.rs:487

### Scenario: A turn with a prior session id resumes that session

- **GIVEN** a turn request carrying a previously-assigned session id
- **WHEN** the harness runs the turn
- **THEN** it opens the session by resuming that same id

> test: code
> - crates/duckchat/src/grok/acp.rs:527

### Scenario: A second turn on a hot path reuses the process

- **GIVEN** a completed turn that left the main path process-hot

- **WHEN** a second turn is run on the same main path

- **THEN** the harness does not spawn a new `grok agent stdio` process for that turn

- **AND** the turn still opens or resumes the conversation session as required by the
  session id

> test: code
> - crates/duckchat/src/grok/runtime.rs:532

### Scenario: After cancel, the next turn can spawn and resume

- **GIVEN** a main path whose in-flight turn was cancelled (main child killed)
- **AND** a prior conversation session id
- **WHEN** a later turn is run with that session id
- **THEN** the harness may spawn a new process
- **AND** it opens the session by resuming that id

> test: code
> - crates/duckchat/src/grok/runtime.rs:567

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
> - crates/duckchat/src/grok.rs:326

### Scenario: Title model falls back when the preferred fast model is absent

- **GIVEN** a set of available models that does not include the preferred fast model
- **WHEN** the harness selects a model for title summarisation
- **THEN** it selects another available model rather than failing

> test: code
> - crates/duckchat/src/grok.rs:344

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
> - crates/duckchat/src/grok.rs:359

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
> - crates/duckchat/src/grok.rs:259

### Scenario: Surrounding text is preserved as text blocks

- **GIVEN** a prompt with text before and after a resolved image `attach:` marker

- **WHEN** the harness assembles the prompt for the turn

- **THEN** the text before the marker appears as a text content block before the image
  block

- **AND** the text after the marker appears as a text content block after the image block

> test: code
> - crates/duckchat/src/grok.rs:279

### Scenario: A non-image attachment is represented as text

- **GIVEN** a turn whose prompt contains an `attach:` link
- **AND** the turn's attachments map holds a non-image payload for that link's id
- **WHEN** the harness assembles the prompt for the turn
- **THEN** the attachment is represented as a text content block
- **AND** the content does not include an image content block for that attachment

> test: code
> - crates/duckchat/src/grok.rs:295

### Scenario: An unresolved attach marker is left literal

- **GIVEN** a turn whose prompt contains an `attach:` link
- **AND** the turn's attachments map has no entry for that link's id
- **WHEN** the harness assembles the prompt for the turn
- **THEN** the original markdown link remains as text content

> test: code
> - crates/duckchat/src/grok.rs:317

## Requirement: Warm oneshot path

Title summary and reply-suggestion calls on the grok oneshot path SHALL reuse a warm
oneshot process when the path is already process-hot, rather than spawning a new
`grok agent stdio` child for each call. Each oneshot call SHALL use a fresh grok ACP
session (N=1) and SHALL NOT resume a prior oneshot conversation session.

> test: code

### Scenario: A second oneshot call does not resume the prior oneshot session

- **GIVEN** a grok oneshot path that has completed one oneshot call
- **WHEN** a second oneshot call is made on that path
- **THEN** the second call opens a fresh grok session
- **AND** it does not resume the prior oneshot session id

> test: code
> - crates/duckchat/src/grok/runtime.rs:628

### Scenario: An oneshot call on a hot path reuses the process

- **GIVEN** a grok oneshot path that is already process-hot
- **WHEN** an oneshot call is made on that path
- **THEN** the harness does not spawn a new `grok agent stdio` process for that call

> test: code
> - crates/duckchat/src/grok/runtime.rs:660
