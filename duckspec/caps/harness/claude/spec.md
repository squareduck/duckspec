# Claude harness

The Claude harness drives Claude Code through an owned ACP agent child over the official
`claude` CLI: the official process starts on the first user prompt (not at open), sessions
bind Claude's native ids for resume, the main path keeps a duplex-hot Claude process when
possible after that bind, and the agent streams profile-compatible `session/update`
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
> - crates/duckchat/src/claude_code.rs:244

### Scenario: The agent uses the official claude CLI as its backend

- **GIVEN** the owned Claude ACP agent handling a turn
- **WHEN** it executes the turn against Claude Code
- **THEN** the backend process is the official `claude` CLI

> test: code
> - crates/duckchat-claude-acp/src/claude/spawn.rs:105

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
> - crates/duckchat-claude-acp/src/agent.rs:681

### Scenario: A turn without a prior session opens a new session and surfaces Claude's native session id

- **GIVEN** a Claude turn request carrying no session id
- **WHEN** the harness runs the turn
- **THEN** it opens a fresh Claude session
- **AND** it surfaces Claude Code's native session id

> test: code
> - crates/duckchat-claude-acp/src/agent.rs:718

### Scenario: A turn with a prior Claude session id resumes that id

- **GIVEN** a Claude turn request carrying a previously assigned Claude session id
- **WHEN** the harness runs the turn
- **THEN** it opens the session by resuming that same id

> test: code
> - crates/duckchat-claude-acp/src/agent.rs:756

## Requirement: Duplex main heat

When the Claude main path is duplex-hot, a subsequent main turn SHALL reuse the inner
`claude` process rather than spawning a new one for that turn. Cancelling an in-flight
Claude turn SHALL end that heat; a later turn SHALL be allowed to start Claude again and,
when a prior session id is supplied, resume that id.

> test: code

### Scenario: A second main turn reuses the inner Claude process when duplex-hot

- **GIVEN** a completed Claude main turn that left the inner Claude process duplex-hot

- **WHEN** a second main turn is run on the same path

- **THEN** the agent does not spawn a new `claude` process for that turn

- **AND** the turn still opens or resumes the conversation session as required by the
  session id

> test: code
> - crates/duckchat-claude-acp/src/agent.rs:809

### Scenario: After cancel, a later turn may start Claude again and resume a prior session id

- **GIVEN** a Claude main path whose in-flight turn was cancelled
- **AND** a prior Claude conversation session id
- **WHEN** a later turn is run with that session id
- **THEN** the agent may start a new `claude` process
- **AND** it opens the session by resuming that id

> test: code
> - crates/duckchat-claude-acp/src/agent.rs:863

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
> - crates/duckchat-claude-acp/src/claude/map.rs:179

### Scenario: Profile updates are delivered to the host before the turn completes

- **GIVEN** Claude producing a profile-mapped update during a turn (for example assistant
  text)

- **WHEN** the agent is still running that turn

- **THEN** the host receives the corresponding profile `session/update` before the turn's
  prompt result

> test: code
> - crates/duckchat-claude-acp/src/agent.rs:938

### Scenario: A Claude tool call surfaces as profile tool use then result

- **GIVEN** Claude performing a tool call and completing it during a turn

- **WHEN** the agent translates the stream for the ACP client

- **THEN** it emits a profile tool-call update with the call's id, name, and input

- **AND** it emits a completed profile tool-call update carrying the same call id and the
  tool output

> test: code
> - crates/duckchat-claude-acp/src/claude/map.rs:200

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
> - crates/duckchat/src/claude_code/agent_bin.rs:136

### Scenario: When env is unset, a sibling of the running executable is used if present

- **GIVEN** no environment override for the Claude ACP agent
- **AND** a Claude ACP agent binary next to the running executable
- **WHEN** the Claude harness resolves the agent to spawn
- **THEN** it selects that sibling binary

> test: code
> - crates/duckchat/src/claude_code/agent_bin.rs:150

### Scenario: A missing agent binary fails the turn with a typed error

- **GIVEN** no resolvable Claude ACP agent binary
- **WHEN** a Claude turn is run
- **THEN** the turn fails with a typed error rather than panicking

> test: code
> - crates/duckchat/src/claude_code/agent_bin.rs:167

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
> - crates/duckchat-claude-acp/src/claude/spawn.rs:146

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
> - crates/duckchat-claude-acp/src/claude/ask_user.rs:302

### Scenario: Host selection completes with allow and answers

- **GIVEN** a pending AskUserQuestion exposed as a host user choice

- **WHEN** the host answers with a selected option label for the question

- **THEN** Claude's request is completed as allow

- **AND** the updated input includes an answers entry mapping that question text to that
  label

> test: code
> - crates/duckchat-claude-acp/src/claude/ask_user.rs:221

### Scenario: Host custom freeform answer completes with allow and free-text answers

- **GIVEN** a pending AskUserQuestion exposed as a host user choice

- **AND** a question text from that request

- **WHEN** the host answers with custom freeform text

- **THEN** Claude's request is completed as allow

- **AND** the updated input includes an answers entry mapping that question text to that
  freeform text

- **AND** the request is not completed as deny

> test: code
> - crates/duckchat-claude-acp/src/claude/ask_user.rs:249

### Scenario: Host cancel completes without accepting the questionnaire

- **GIVEN** a pending AskUserQuestion exposed as a host user choice
- **WHEN** the host answers as cancelled
- **THEN** Claude's request is completed without accepting the questionnaire

> test: code
> - crates/duckchat-claude-acp/src/claude/ask_user.rs:274

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
> - crates/duckchat-claude-acp/src/claude/ask_user.rs:288

## Requirement: Oneshot preferred model

Title-summary and reply-suggestion oneshots on the Claude harness SHALL select the
preferred cheap/fast model (the curated `haiku` alias) when that model is among the models
the agent advertises. When the preferred model is not advertised, those oneshots SHALL
select another advertised model rather than failing. Main conversation turns SHALL NOT be
required to use this preferred oneshot model (session model selection is separate).

> test: code

### Scenario: Preferred oneshot model is selected when advertised

- **GIVEN** the Claude agent advertising available models that include the preferred
  oneshot model among others

- **WHEN** the harness selects a model for a title-summary or reply-suggestion oneshot

- **THEN** it selects the preferred oneshot model

> test: code
> - crates/duckchat/src/acp/runtime.rs:971

### Scenario: Oneshot model falls back when preferred is absent

- **GIVEN** the Claude agent advertising available models that do not include the
  preferred oneshot model

- **WHEN** the harness selects a model for a title-summary or reply-suggestion oneshot

- **THEN** it selects another advertised model rather than failing

> test: code
> - crates/duckchat/src/acp/runtime.rs:987
