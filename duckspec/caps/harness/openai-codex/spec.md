# OpenAI Codex harness

The OpenAI Codex harness drives Codex through an owned ACP agent child over the official
`codex app-server`: the host uses the shared ACP client, sessions bind Codex thread ids
for resume, the agent keeps the app-server process warm across main turns when possible,
and the agent streams profile-compatible `session/update` notifications for the shared
client.

## Requirement: Owned ACP agent over official Codex

A Codex turn SHALL be driven by the shared ACP client against the owned Codex ACP agent
process, not by an in-host Codex App Server client. That agent SHALL use the official
`codex app-server` as its backend. The harness SHALL NOT require npm, Node, or another
foreign runtime to run Codex turns.

> test: code

### Scenario: A Codex turn is driven through the owned ACP agent process

- **GIVEN** a turn whose model names the openai-codex harness
- **WHEN** the turn runs
- **THEN** the host ACP client speaks to the owned Codex ACP agent process
- **AND** the host does not drive Codex via an in-host App Server client

> test: code
> - crates/duckchat/src/openai_codex.rs:302

### Scenario: The agent uses official codex app-server as its backend

- **GIVEN** the owned Codex ACP agent handling a turn
- **WHEN** it executes the turn against Codex
- **THEN** the backend process is the official `codex app-server`

> test: code
> - crates/duckchat-codex-acp/src/codex/spawn.rs:58

### Scenario: The harness does not require a Node or npm runtime

- **GIVEN** the owned Codex ACP agent binary and the official `codex` backend
- **WHEN** a Codex turn is run
- **THEN** the turn does not depend on Node, npm, or an npx-launched adapter process

> test: code
> - crates/duckchat/src/openai_codex/agent_bin.rs:206

## Requirement: Session lifecycle and thread ids

Completing a turn that opened without a prior session id SHALL surface a Codex thread id
for the host to persist. Running a turn with a prior session id SHALL resume that same id.
When resuming a session id the agent reports as missing, the outcome SHALL be
session-not-found so the host can drop the id and retry.

> test: code

### Scenario: A turn without a prior session opens a new session and surfaces a Codex thread id

- **GIVEN** a Codex turn request carrying no session id
- **WHEN** the harness runs the turn
- **THEN** it opens a fresh Codex session
- **AND** it surfaces a Codex thread id

> test: code
> - crates/duckchat-codex-acp/src/agent.rs:706

### Scenario: A turn with a prior session id resumes that id

- **GIVEN** a Codex turn request carrying a previously assigned Codex session id
- **WHEN** the harness runs the turn
- **THEN** it opens the session by resuming that same id

> test: code
> - crates/duckchat-codex-acp/src/agent.rs:726

### Scenario: A failed load of a missing session surfaces session-not-found

- **GIVEN** a Codex turn request carrying a session id the agent cannot load
- **WHEN** the harness opens the session
- **THEN** the outcome is session-not-found rather than a successful resume

> test: code
> - crates/duckchat-codex-acp/src/agent.rs:750

## Requirement: App-server process heat

When the Codex main path is process-hot, a subsequent main turn SHALL reuse the app-server
process rather than spawning a new one for that turn. Cancelling an in-flight Codex turn
SHALL end that heat; a later turn SHALL be allowed to start the app-server again and, when
a prior session id is supplied, resume that id.

> test: code

### Scenario: A second main turn reuses the app-server process when hot

- **GIVEN** a completed Codex main turn that left the app-server process hot

- **WHEN** a second main turn is run on the same path

- **THEN** the agent does not spawn a new app-server process for that turn

- **AND** the turn still opens or resumes the conversation session as required by the
  session id

> test: code
> - crates/duckchat-codex-acp/src/agent.rs:770

### Scenario: After cancel, a later turn may spawn again and resume a prior session id

- **GIVEN** a Codex main path whose in-flight turn was cancelled
- **AND** a prior Codex conversation session id
- **WHEN** a later turn is run with that session id
- **THEN** the agent may start a new app-server process
- **AND** it opens the session by resuming that id

> test: code
> - crates/duckchat-codex-acp/src/agent.rs:792

## Requirement: Profile-compatible event emission

The Codex ACP agent SHALL emit profile `session/update` notifications so the shared ACP
client can map them to neutral agent events: assistant text as content updates, a tool
invocation as a tool-use update followed by a completed result update sharing the same
call id, and token telemetry as a usage update carrying total tokens used. While a turn is
in progress, the agent SHALL deliver those profile updates to the host as they become
available, rather than only after the turn has completed.

> test: code

### Scenario: Assistant text surfaces as profile content updates

- **GIVEN** Codex streaming assistant text during a turn
- **WHEN** the agent translates the stream for the ACP client
- **THEN** it emits profile assistant message chunks for that text

> test: code
> - crates/duckchat-codex-acp/src/codex/map.rs:246

### Scenario: A tool call surfaces as profile tool use then completed result

- **GIVEN** Codex performing a tool call and completing it during a turn

- **WHEN** the agent translates the stream for the ACP client

- **THEN** it emits a profile tool-call update with the call's id, name, and input

- **AND** it emits a completed profile tool-call update carrying the same call id and the
  tool output

> test: code
> - crates/duckchat-codex-acp/src/codex/map.rs:268

### Scenario: Token telemetry surfaces as usage with total tokens

- **GIVEN** Codex reporting token usage during a turn
- **WHEN** the agent translates that telemetry for the ACP client
- **THEN** it emits a usage update carrying the total tokens used

> test: code
> - crates/duckchat-codex-acp/src/codex/map.rs:321

## Requirement: Agent binary discovery

Resolving the Codex ACP agent binary SHALL prefer an explicit environment override, then a
binary sibling of the running executable when present, then the process `PATH`. When no
agent binary can be launched, running a Codex turn SHALL fail with a typed error rather
than panicking.

> test: code

### Scenario: An explicit env override selects the agent binary

- **GIVEN** an environment override naming a Codex ACP agent binary
- **WHEN** the Codex harness resolves the agent to spawn
- **THEN** it selects that override path

> test: code
> - crates/duckchat/src/openai_codex/agent_bin.rs:135

### Scenario: When env is unset, a sibling of the running executable is used if present

- **GIVEN** no environment override for the Codex ACP agent
- **AND** a Codex ACP agent binary next to the running executable
- **WHEN** the Codex harness resolves the agent to spawn
- **THEN** it selects that sibling binary

> test: code
> - crates/duckchat/src/openai_codex/agent_bin.rs:149

### Scenario: A missing agent binary fails the turn with a typed error

- **GIVEN** no resolvable Codex ACP agent binary
- **WHEN** a Codex turn is run
- **THEN** the turn fails with a typed error rather than panicking

> test: code
> - crates/duckchat/src/openai_codex/agent_bin.rs:166

## Requirement: Mid-turn structured questions

When the Codex backend issues a structured user-input request during a turn, the owned
agent SHALL surface a structured choice to the ACP parent so the host receives a neutral
user-choice event. Completing that choice with a selection SHALL finish the backend
request as accepted with answers for the chosen option. Completing with a custom freeform
answer SHALL finish the backend request as accepted with that freeform text as the answer
(not a skip). Completing as cancelled SHALL finish the backend request without accepting
the questionnaire.

> test: code

### Scenario: A structured user-input request surfaces a host user choice

- **GIVEN** an in-flight Codex main-path turn
- **AND** a structured user-input request with at least one option
- **WHEN** the owned agent handles that request
- **THEN** the ACP parent surfaces a host user-choice event for those options

> test: code
> - crates/duckchat-codex-acp/src/codex/ask_user.rs:284

### Scenario: Host selection completes with accepted answers

- **GIVEN** a pending Codex structured user-input request exposed as a host user choice
- **WHEN** the host answers with a selected option
- **THEN** the backend request is completed as accepted
- **AND** the response carries the chosen answer for the question

> test: code
> - crates/duckchat-codex-acp/src/codex/ask_user.rs:380

### Scenario: Host custom freeform completes with accepted free-text answers

- **GIVEN** a pending Codex structured user-input request exposed as a host user choice

- **AND** a question text from that request

- **WHEN** the host answers with custom freeform text

- **THEN** the backend request is completed as accepted

- **AND** the response carries an answers entry mapping that question text to that
  freeform text

- **AND** the response is not a skip or cancel outcome

> test: code
> - crates/duckchat-codex-acp/src/codex/ask_user.rs:400

### Scenario: Host cancel completes without accepting the questionnaire

- **GIVEN** a pending Codex structured user-input request exposed as a host user choice
- **WHEN** the host answers as cancelled
- **THEN** the backend request is completed without accepting the questionnaire

> test: code
> - crates/duckchat-codex-acp/src/codex/ask_user.rs:411

## Requirement: Ordinary tools stay auto-approved

Non-question tool invocations on the Codex main path SHALL NOT require host UI. Structured
user-input requests remain the structured-choice path for clarifying questions.

> test: code

### Scenario: Ordinary tool permission does not require host UI

- **GIVEN** a Codex main-path turn
- **AND** Codex requesting permission for an ordinary non-question tool
- **WHEN** the owned agent handles that permission request
- **THEN** the tool is allowed without emitting a host user-choice event

> test: code
> - crates/duckchat-codex-acp/src/codex/ask_user.rs:423

## Requirement: Model discovery and oneshot preference

Listing Codex models on the host SHALL return the models the owned Codex agent advertises
on initialize, each tagged with the openai-codex harness. Each listed model SHALL carry a
human-readable display name. Title-summary and reply-suggestion oneshots SHALL select the
preferred oneshot model for the openai-codex harness when that model is among the
available models, and SHALL fall back to another available model when the preferred model
is absent.

> test: code

### Scenario: Discovered models are tagged with the openai-codex harness

- **GIVEN** the owned Codex agent advertising its available models
- **WHEN** the harness lists models
- **THEN** each returned model is tagged with the openai-codex harness

> test: code
> - crates/duckchat-codex-acp/src/models.rs:136
> - crates/duckchat/src/openai_codex.rs:365

### Scenario: Each listed model carries a display name

- **GIVEN** the owned Codex agent advertising its available models
- **WHEN** the harness lists models
- **THEN** each returned model carries a non-empty display name

> test: code
> - crates/duckchat/src/openai_codex.rs:388

### Scenario: Preferred oneshot model is selected when advertised

- **GIVEN** the Codex agent advertising available models that include the preferred
  oneshot model for the openai-codex harness among others

- **WHEN** the harness selects a model for a title-summary or reply-suggestion oneshot

- **THEN** it selects the preferred oneshot model

> test: code
> - crates/duckchat/src/openai_codex.rs:409

### Scenario: Oneshot model falls back when preferred is absent

- **GIVEN** the Codex agent advertising available models that do not include the preferred
  oneshot model for the openai-codex harness

- **WHEN** the harness selects a model for a title-summary or reply-suggestion oneshot

- **THEN** it selects another advertised model rather than failing

> test: code
> - crates/duckchat/src/openai_codex.rs:421

## Requirement: Prompt attachments

When assembling a turn for the Codex backend, the harness path SHALL walk the folded
prompt text for markdown links of the form `[label](attach:<id>)`, resolve each link
against the turn's attachments map, and deliver multi-part turn input. A resolved image
attachment SHALL appear as a local image input on the turn carrying that attachment's
image bytes. Surrounding text SHALL appear as text inputs. An unresolved `attach:` link
SHALL be left as its original literal markdown text.

> test: code

### Scenario: A resolved image attachment is delivered as a local image input on the turn

- **GIVEN** a turn whose prompt contains an `attach:` link
- **AND** the turn's attachments map holds an image payload for that link's id
- **WHEN** the harness assembles the turn input
- **THEN** the turn input includes a local image input for that attachment

> test: code
> - crates/duckchat-codex-acp/src/codex/content.rs:130

### Scenario: Surrounding text is preserved as text inputs

- **GIVEN** a prompt with text before and after a resolved image `attach:` marker
- **WHEN** the harness assembles the turn input
- **THEN** the text before the marker appears as a text input before the image input
- **AND** the text after the marker appears as a text input after the image input

> test: code
> - crates/duckchat-codex-acp/src/codex/content.rs:153

### Scenario: An unresolved attach marker is left literal

- **GIVEN** a turn whose prompt contains an `attach:` link
- **AND** the turn's attachments map has no entry for that link's id
- **WHEN** the harness assembles the turn input
- **THEN** the original markdown link remains as text input

> test: code
> - crates/duckchat-codex-acp/src/codex/content.rs:175

## Requirement: Graceful unavailability

When the Codex ACP agent binary, the official Codex backend, or its authentication is
unavailable, listing models SHALL return an empty list and running a turn SHALL fail with
a typed error rather than panicking.

> test: code

### Scenario: A missing agent or backend yields no models and a turn error

- **GIVEN** an environment where the Codex ACP agent or official Codex backend cannot be
  launched

- **WHEN** the harness lists models and then runs a turn

- **THEN** the model list is empty

- **AND** the turn fails with a typed error rather than panicking

> test: code
> - crates/duckchat-codex-acp/src/agent.rs:676
> - crates/duckchat/src/openai_codex.rs:434

## Requirement: Stage skill discovery

Listing slash commands for the openai-codex harness SHALL discover stage skills under
`.agents/skills` in the project root: each skill directory that contains a `SKILL.md` with
a usable name contributes one command. When no such skills are present, the command list
SHALL be empty without failing.

> test: code

### Scenario: Skills under .agents/skills are listed as slash commands

- **GIVEN** a project root containing `.agents/skills` skill directories with `SKILL.md`
  files

- **WHEN** the openai-codex harness lists commands for that project

- **THEN** each skill is present in the command list with its skill name

> test: code
> - crates/duckchat/src/openai_codex/discover.rs:105

### Scenario: A project without .agents/skills yields an empty command list

- **GIVEN** a project root with no `.agents/skills` tree
- **WHEN** the openai-codex harness lists commands for that project
- **THEN** the command list is empty

> test: code
> - crates/duckchat/src/openai_codex/discover.rs:131
