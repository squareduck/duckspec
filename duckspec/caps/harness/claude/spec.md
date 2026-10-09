# Claude harness

The Claude harness drives Claude Code through an owned ACP agent child over the official
`claude` CLI: the official process starts on the first user prompt (not at open), sessions
bind Claude's native ids for resume, the main path keeps a duplex-hot Claude process when
possible after that bind, and the agent streams profile-compatible `session/update`
notifications (text, tools, and thinking) for the shared ACP client.

The Claude harness drives Claude Code through an owned ACP agent child over the official
`claude` CLI: the official process starts on the first user prompt (not at open), sessions
bind Claude's native ids for resume, the main path keeps a duplex-hot Claude process after
that bind while the prompt's model and effort still match, the agent advertises the Claude
Code catalog on initialize, and the agent streams profile-compatible `session/update`
notifications (text, tools, and thinking) for the shared ACP client.

## Requirement: Owned ACP agent over official Claude CLI

A Claude turn SHALL be driven by the shared ACP client against the owned Claude ACP agent
process, not by an in-host stream-json client. That agent SHALL use the official `claude`
CLI as its backend. The harness SHALL NOT require npm, Node, or another foreign runtime to
run Claude turns.

> test: code

### Scenario: A Claude turn is driven through the owned ACP agent process

- **GIVEN** a turn whose model names the Claude harness
- **WHEN** the turn runs
- **THEN** the host ACP client speaks to the owned Claude ACP agent process
- **AND** the host does not drive Claude via an in-host stream-json client

> test: code
> - crates/duckchat/src/claude_code.rs:356

### Scenario: The agent uses the official claude CLI as its backend

- **GIVEN** the owned Claude ACP agent handling a turn
- **WHEN** it executes the turn against Claude Code
- **THEN** the backend process is the official `claude` CLI

> test: code
> - crates/duckchat-claude-acp/src/claude/spawn.rs:113

## Requirement: Session lifecycle and native session ids

Opening a new Claude conversation without a prior session id SHALL NOT start the official
`claude` process before the first user prompt is submitted. Completing a turn that opened
without a prior session id SHALL surface Claude Code's native session id for the host to
persist. Running a turn with a prior session id SHALL resume that same id. After the first
prompt binds a native id, the ACP session id the host persists for resume SHALL be that
Claude Code native session id.

> test: code

### Scenario: Opening a new session does not start the official claude process before the first user prompt

- **GIVEN** a Claude conversation with no prior session id
- **WHEN** the harness opens a new session without submitting user content
- **THEN** the open completes without starting the official `claude` process

> test: code
> - crates/duckchat-claude-acp/src/agent.rs:756

### Scenario: A turn without a prior session opens a new session and surfaces Claude's native session id

- **GIVEN** a Claude turn request carrying no session id
- **WHEN** the harness runs the turn
- **THEN** it opens a fresh Claude session
- **AND** it surfaces Claude Code's native session id

> test: code
> - crates/duckchat-claude-acp/src/agent.rs:793

### Scenario: A turn with a prior Claude session id resumes that id

- **GIVEN** a Claude turn request carrying a previously assigned Claude session id
- **WHEN** the harness runs the turn
- **THEN** it opens the session by resuming that same id

> test: code
> - crates/duckchat-claude-acp/src/agent.rs:831

## Requirement: Duplex main heat

The main path reuses the inner `claude` process only while it is still that session and
was spawned with this prompt's model and effort.

> test: code

### Scenario: After cancel, a later turn may start Claude again and resume a prior session id

- **GIVEN** a Claude main path whose in-flight turn was cancelled
- **AND** a prior Claude conversation session id
- **WHEN** a later turn is run with that session id
- **THEN** the agent may start a new `claude` process
- **AND** it opens the session by resuming that id

> test: code
> - crates/duckchat-claude-acp/src/agent.rs:938

### Scenario: Hot process follows the prompt's model and effort

- **GIVEN** a duplex-hot Claude process for a native session, spawned with a model and an
  effort level

- **WHEN** a later main prompt matches that spawn, another changes the model or effort,
  and another has no effort level

- **THEN** the matching prompt reuses the hot process

- **AND** a changed model or effort kills that process and resumes the same native session
  id with `--model` for the prompt

- **AND** `--effort` is passed for a prompt that has a level and omitted for a prompt that
  has none

> test: code
> - crates/duckchat-claude-acp/src/agent.rs:1076
> - crates/duckchat-claude-acp/src/agent.rs:884

### Scenario: A model or effort change during an in-flight turn does not cancel that turn

- **GIVEN** an in-flight Claude main turn
- **WHEN** the chat's model or effort changes
- **THEN** that turn is not cancelled

> test: code
> - crates/duckchat-claude-acp/src/agent.rs:1219

### Scenario: A title or reply oneshot leaves the main hot process in place

- **GIVEN** a duplex-hot Claude main process
- **WHEN** a title-summary or reply-suggestion oneshot runs
- **THEN** that main hot process is still in place
- **AND** the oneshot does not pass an effort flag

> test: code
> - crates/duckchat-claude-acp/src/agent.rs:1302

## Requirement: Profile-compatible event emission

The Claude ACP agent SHALL emit profile `session/update` notifications so the shared ACP
client can map them to neutral agent events: assistant text as content updates, Claude
thinking as thought updates, and a tool invocation as a tool-use update followed by a
completed result update sharing the same call id. While a turn is in progress, the agent
SHALL deliver those profile updates to the host as they become available from Claude,
rather than only after the turn has completed.

> test: code

### Scenario: Assistant text from Claude surfaces as profile content updates

- **GIVEN** Claude streaming assistant text during a turn
- **WHEN** the agent translates the stream for the ACP client
- **THEN** it emits profile assistant message chunks for that text

> test: code
> - crates/duckchat-claude-acp/src/claude/map.rs:158

### Scenario: Claude thinking surfaces as profile thought chunks

- **GIVEN** Claude streaming thinking content during a turn
- **WHEN** the agent translates the stream for the ACP client
- **THEN** it emits profile thought chunks for that thinking

> test: code
> - crates/duckchat-claude-acp/src/claude/map.rs:176

### Scenario: Profile updates are delivered to the host before the turn completes

- **GIVEN** Claude producing a profile-mapped update during a turn (for example assistant
  text)

- **WHEN** the agent is still running that turn

- **THEN** the host receives the corresponding profile `session/update` before the turn's
  prompt result

> test: code
> - crates/duckchat-claude-acp/src/agent.rs:1013

### Scenario: A Claude tool call surfaces as profile tool use then result

- **GIVEN** Claude performing a tool call and completing it during a turn

- **WHEN** the agent translates the stream for the ACP client

- **THEN** it emits a profile tool-call update with the call's id, name, and input

- **AND** it emits a completed profile tool-call update carrying the same call id and the
  tool output

> test: code
> - crates/duckchat-claude-acp/src/claude/map.rs:194

## Requirement: Agent binary discovery

Resolving the Claude ACP agent binary SHALL prefer an explicit environment override, then
a binary sibling of the running executable when present, then the process `PATH`. When no
agent binary can be launched, running a Claude turn SHALL fail with a typed error rather
than panicking.

> test: code

### Scenario: An explicit env override selects the agent binary

- **GIVEN** an environment override naming a Claude ACP agent binary
- **WHEN** the Claude harness resolves the agent to spawn
- **THEN** it selects that override path

> test: code
> - crates/duckchat/src/claude_code/agent_bin.rs:135

### Scenario: When env is unset, a sibling of the running executable is used if present

- **GIVEN** no environment override for the Claude ACP agent
- **AND** a Claude ACP agent binary next to the running executable
- **WHEN** the Claude harness resolves the agent to spawn
- **THEN** it selects that sibling binary

> test: code
> - crates/duckchat/src/claude_code/agent_bin.rs:149

### Scenario: A missing agent binary fails the turn with a typed error

- **GIVEN** no resolvable Claude ACP agent binary
- **WHEN** a Claude turn is run
- **THEN** the turn fails with a typed error rather than panicking

> test: code
> - crates/duckchat/src/claude_code/agent_bin.rs:166

## Requirement: AskUserQuestion available

The owned Claude ACP agent SHALL NOT list `AskUserQuestion` among tools disallowed for the
official `claude` backend, so Claude may issue structured clarifying questions during a
turn.

> test: code

### Scenario: AskUserQuestion is not among disallowed tools

- **GIVEN** the owned Claude ACP agent's backend launch configuration
- **WHEN** the disallowed-tools list is inspected
- **THEN** it does not include `AskUserQuestion`

> test: code
> - crates/duckchat-claude-acp/src/claude/spawn.rs:154

## Requirement: Mid-prompt parent choice

When Claude issues an `AskUserQuestion` request during a turn (via the stream-json control
/ canUseTool path), the owned agent SHALL surface a structured choice to the ACP parent so
the host receives a neutral user-choice event. Completing that choice with a selection
SHALL finish Claude's request as allow with an answers map from question text to selected
option label. Completing with a custom freeform answer SHALL finish Claude's request as
allow with an answers map from question text to that freeform text (not deny). Completing
as cancelled SHALL finish Claude's request without accepting the questionnaire (deny or
equivalent skip).

> test: code

### Scenario: An AskUserQuestion request surfaces a host user choice

- **GIVEN** an in-flight Claude main-path turn
- **AND** Claude issuing an AskUserQuestion with at least one option
- **WHEN** the owned agent handles that request
- **THEN** the ACP parent surfaces a host user-choice event for those options

> test: code
> - crates/duckchat-claude-acp/src/claude/ask_user.rs:290

### Scenario: Host selection completes with allow and answers

- **GIVEN** a pending AskUserQuestion exposed as a host user choice

- **WHEN** the host answers with a selected option label for the question

- **THEN** Claude's request is completed as allow

- **AND** the updated input includes an answers entry mapping that question text to that
  label

> test: code
> - crates/duckchat-claude-acp/src/claude/ask_user.rs:207

### Scenario: Host custom freeform answer completes with allow and free-text answers

- **GIVEN** a pending AskUserQuestion exposed as a host user choice

- **AND** a question text from that request

- **WHEN** the host answers with custom freeform text

- **THEN** Claude's request is completed as allow

- **AND** the updated input includes an answers entry mapping that question text to that
  freeform text

- **AND** the request is not completed as deny

> test: code
> - crates/duckchat-claude-acp/src/claude/ask_user.rs:235

### Scenario: Host cancel completes without accepting the questionnaire

- **GIVEN** a pending AskUserQuestion exposed as a host user choice
- **WHEN** the host answers as cancelled
- **THEN** Claude's request is completed without accepting the questionnaire

> test: code
> - crates/duckchat-claude-acp/src/claude/ask_user.rs:260

## Requirement: Ordinary tools stay auto-approved

Non-question tool invocations on the Claude main path SHALL NOT require host UI when the
backend is configured for permission bypass of ordinary tools. AskUserQuestion remains the
structured-choice path for clarifying questions.

> test: code

### Scenario: Non-question tools do not require host UI under bypass

- **GIVEN** a Claude main-path turn with ordinary-tool permission bypass enabled
- **AND** Claude invoking a non-question tool that is not AskUserQuestion
- **WHEN** the owned agent handles that tool permission
- **THEN** the tool is allowed without emitting a host user-choice event

> test: code
> - crates/duckchat-claude-acp/src/claude/ask_user.rs:274

## Requirement: Oneshot preferred model

Title-summary and reply-suggestion oneshots on the Claude harness SHALL select the
preferred oneshot model for that harness when that model is among the models the agent
advertises. When the preferred model is not advertised, those oneshots SHALL select
another advertised model rather than failing. Main conversation turns SHALL NOT be
required to use this preferred oneshot model (session model selection is separate).

> test: code

### Scenario: Preferred oneshot model is selected when advertised

- **GIVEN** the Claude agent advertising available models that include the preferred
  oneshot model for that harness among others

- **WHEN** the harness selects a model for a title-summary or reply-suggestion oneshot

- **THEN** it selects the preferred oneshot model

> test: code
> - crates/duckchat/src/acp/runtime.rs:1145

### Scenario: Oneshot model falls back when preferred is absent

- **GIVEN** the Claude agent advertising available models that do not include the
  preferred oneshot model

- **WHEN** the harness selects a model for a title-summary or reply-suggestion oneshot

- **THEN** it selects another advertised model rather than failing

> test: code
> - crates/duckchat/src/acp/runtime.rs:1161

## Requirement: Model discovery

The host's Claude model list is the set the agent advertised on initialize, carried
through with the same name, window, and effort.

> test: code

### Scenario: Listed models follow the agent advertise set

- **GIVEN** the owned Claude agent advertising two models on initialize
- **AND** one model has a display name, a context window, and an effort scale
- **AND** the other model has a display name and neither a window nor an effort scale
- **WHEN** the harness lists models
- **THEN** the listed models are exactly that advertised set
- **AND** each listed model is tagged with the Claude harness
- **AND** the first model carries that display name, context window, and effort scale
- **AND** the second model carries its display name and no context window or effort scale

> test: code
> - crates/duckchat/src/claude_code.rs:430

## Requirement: Agent model advertise

Initialize advertises the current Claude Code catalog's selectable rows and reports
whether that fetch succeeded.

> test: code

### Scenario: Admitted catalog rows carry name, window, and effort

- **GIVEN** a Claude Code catalog and a `claude` binary whose leading version triple
  satisfies one main row and is older than another main row's minimum

- **AND** the satisfied main row has a display name, a context window, and an effort scale

- **AND** another main row the version satisfies has no effort scale

- **AND** the catalog also contains a row outside the main section

- **WHEN** the agent completes initialize

- **THEN** the satisfied row is advertised with that display name, that context window,
  and that effort scale's levels in catalog order plus its default level

- **AND** the row with no effort scale is advertised without effort

- **AND** the too-new row and the non-main row are not advertised

> test: code
> - crates/duckchat-claude-acp/src/models.rs:355

### Scenario: An unreadable binary version drops rows that require a minimum

- **GIVEN** a `claude` binary whose version cannot be read as a leading `N.N.N`

- **AND** a catalog with a main row that requires a minimum version and a main row that
  does not

- **WHEN** the agent completes initialize

- **THEN** the row without a minimum is advertised

- **AND** the row that requires a minimum is not advertised

> test: code
> - crates/duckchat-claude-acp/src/models.rs:424

### Scenario: A failed catalog fetch advertises no models

- **GIVEN** a catalog fetch that fails
- **WHEN** the agent completes initialize
- **THEN** initialize succeeds
- **AND** the catalog is reported failed
- **AND** no models are advertised

> test: code
> - crates/duckchat-claude-acp/src/models.rs:459

### Scenario: Catalog expiry is reported only when the document has one

- **GIVEN** one successful catalog whose document includes an expiry time
- **AND** another successful catalog whose document includes none
- **WHEN** the agent completes initialize for each catalog
- **THEN** both report the catalog ok
- **AND** only the document that has an expiry time is reported with that expiry

> test: code
> - crates/duckchat-claude-acp/src/models.rs:476
