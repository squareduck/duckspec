# Grok harness

The grok harness drives the grok CLI over ACP, starting or resuming a session per turn and
translating grok's `session/update` stream into duckchat's neutral agent events, including
accurate context usage.

The grok harness drives the official grok CLI as a native ACP agent under the shared ACP
client: launch, model discovery, attachments, and oneshot isolation stay Grok-specific;
session open/resume, process heat of the agent child, and profile event mapping are owned
by the shared client.

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
> - crates/duckchat/src/grok.rs:322

### Scenario: Title model falls back when the preferred fast model is absent

- **GIVEN** a set of available models that does not include the preferred fast model
- **WHEN** the harness selects a model for title summarisation
- **THEN** it selects another available model rather than failing

> test: code
> - crates/duckchat/src/grok.rs:340

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
> - crates/duckchat/src/grok.rs:355

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
> - crates/duckchat/src/grok.rs:255

### Scenario: Surrounding text is preserved as text blocks

- **GIVEN** a prompt with text before and after a resolved image `attach:` marker

- **WHEN** the harness assembles the prompt for the turn

- **THEN** the text before the marker appears as a text content block before the image
  block

- **AND** the text after the marker appears as a text content block after the image block

> test: code
> - crates/duckchat/src/grok.rs:275

### Scenario: A non-image attachment is represented as text

- **GIVEN** a turn whose prompt contains an `attach:` link
- **AND** the turn's attachments map holds a non-image payload for that link's id
- **WHEN** the harness assembles the prompt for the turn
- **THEN** the attachment is represented as a text content block
- **AND** the content does not include an image content block for that attachment

> test: code
> - crates/duckchat/src/grok.rs:291

### Scenario: An unresolved attach marker is left literal

- **GIVEN** a turn whose prompt contains an `attach:` link
- **AND** the turn's attachments map has no entry for that link's id
- **WHEN** the harness assembles the prompt for the turn
- **THEN** the original markdown link remains as text content

> test: code
> - crates/duckchat/src/grok.rs:313

## Requirement: Warm oneshot path

Title summary and reply-suggestion calls on the grok oneshot path SHALL reuse a warm
oneshot process when the path is already process-hot, rather than spawning a new agent
process for each call. Each oneshot call SHALL use a fresh grok ACP session (N=1) and
SHALL NOT resume a prior oneshot conversation session.

> test: code

### Scenario: A second oneshot call does not resume the prior oneshot session

- **GIVEN** a grok oneshot path that has completed one oneshot call
- **WHEN** a second oneshot call is made on that path
- **THEN** the second call opens a fresh grok session
- **AND** it does not resume the prior oneshot session id

> test: code
> - crates/duckchat/src/acp/runtime.rs:852

### Scenario: An oneshot call on a hot path reuses the process

- **GIVEN** a grok oneshot path that is already process-hot
- **WHEN** an oneshot call is made on that path
- **THEN** the harness does not spawn a new agent process for that call

> test: code
> - crates/duckchat/src/acp/runtime.rs:884

## Requirement: Native Grok agent launch

A Grok turn SHALL be driven by the shared ACP client against the native grok ACP agent
(the official `grok` CLI in agent stdio mode). The harness SHALL NOT insert an
intermediate owned proxy whose only role is to forward ACP to grok.

> test: code

### Scenario: A Grok turn spawns the native grok ACP agent

- **GIVEN** a turn whose model names the grok harness
- **WHEN** the turn runs
- **THEN** the shared ACP client spawns the native grok ACP agent
- **AND** it does not route the turn through an intermediate Grok-only ACP proxy

> test: code
> - crates/duckchat/src/grok.rs:372

## Requirement: Structured questions enabled

The main-path Grok agent launch SHALL NOT pass `--no-ask-user`, so the agent may issue
structured user questions. The same launch SHALL still auto-approve tool execution for the
turn (always-approve style), so ordinary tool permission prompts do not require host UI.

> test: code

### Scenario: Main launch does not pass no-ask-user

- **GIVEN** the Grok main-path agent launch
- **WHEN** the launch arguments are inspected
- **THEN** they do not include `--no-ask-user`

> test: code
> - crates/duckchat/src/grok.rs:419

### Scenario: Main launch still auto-approves tool execution

- **GIVEN** the Grok main-path agent launch
- **WHEN** the launch arguments are inspected
- **THEN** they include the always-approve flag that auto-approves tool execution

> test: code
> - crates/duckchat/src/grok.rs:429

## Requirement: Question wire mapping

When the Grok agent issues a mid-turn `x.ai/ask_user_question` request, the harness path
SHALL expose that request to the host as a neutral user choice (via the shared ACP client
main path). A host selection SHALL complete the request with an accepted questionnaire
response carrying the chosen answers. A host custom freeform answer SHALL complete the
request with an accepted questionnaire response carrying that freeform text as the answer
value for the question (not skip-interview). A host cancel SHALL complete the request with
a skip-interview response.

> test: code

### Scenario: An ask-user extension request is exposed as a host user choice

- **GIVEN** an in-flight Grok main-path turn
- **AND** an agent `x.ai/ask_user_question` request with at least one option
- **WHEN** the request is handled on the main path
- **THEN** a host user-choice event is emitted for that request

> test: code
> - crates/duckchat/src/grok.rs:439

### Scenario: A host selection completes with an accepted questionnaire response

- **GIVEN** a pending Grok ask-user request exposed as a host user choice
- **WHEN** the host answers with a selected option
- **THEN** the agent request is completed with an accepted questionnaire response
- **AND** that response carries the chosen answer for the question

> test: code
> - crates/duckchat/src/grok.rs:473

### Scenario: Host custom freeform answer completes with an accepted free-text answer

- **GIVEN** a pending Grok ask-user request exposed as a host user choice

- **AND** a question text from that request

- **WHEN** the host answers with custom freeform text

- **THEN** the agent request is completed with an accepted questionnaire response

- **AND** that response carries an answers entry mapping that question text to that
  freeform text

- **AND** the response is not a skip-interview outcome

> test: code
> - crates/duckchat/src/acp/ask_user.rs:104
> - crates/duckchat/src/grok.rs:494

### Scenario: A host cancel completes with a skip-interview response

- **GIVEN** a pending Grok ask-user request exposed as a host user choice
- **WHEN** the host answers as cancelled
- **THEN** the agent request is completed with a skip-interview response

> test: code
> - crates/duckchat/src/grok.rs:483
