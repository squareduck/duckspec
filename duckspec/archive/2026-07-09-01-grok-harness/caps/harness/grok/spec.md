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

### Scenario: A turn with a prior session id resumes that session

- **GIVEN** a turn request carrying a previously-assigned session id
- **WHEN** the harness runs the turn
- **THEN** it opens the session by resuming that same id

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

### Scenario: A tool call surfaces as a use then a matching result

- **GIVEN** a session update stream containing a tool call and its completion
- **WHEN** the harness translates the stream
- **THEN** a tool-use event is emitted with the call's id, name, and input
- **AND** a tool-result event is emitted carrying the same call id and the tool output

### Scenario: A usage update carries used tokens and the model's context window

- **GIVEN** a session update reporting a running total-token count
- **AND** an active model whose context window is known
- **WHEN** the harness translates the update
- **THEN** a usage event is emitted with that used-token count and that context window

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

### Scenario: Title model falls back when the preferred fast model is absent

- **GIVEN** a set of available models that does not include the preferred fast model
- **WHEN** the harness selects a model for title summarisation
- **THEN** it selects another available model rather than failing

## Requirement: Graceful unavailability

When the grok binary or its authentication is unavailable, listing models SHALL return an
empty list and running a turn SHALL fail with a typed error rather than panicking.

> test: code

### Scenario: A missing grok binary yields no models and a turn error

- **GIVEN** an environment where the grok binary cannot be launched
- **WHEN** the harness lists models and then runs a turn
- **THEN** the model list is empty
- **AND** the turn fails with a typed error rather than panicking
