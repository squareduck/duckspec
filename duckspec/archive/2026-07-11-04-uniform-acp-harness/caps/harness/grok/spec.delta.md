# @ Grok harness

The grok harness drives the official grok CLI as a native ACP agent under the shared ACP
client: launch, model discovery, attachments, and oneshot isolation stay Grok-specific;
session open/resume, process heat of the agent child, and profile event mapping are owned
by the shared client.

## - Requirement: Session lifecycle and resume

## - Requirement: Event translation

## @ Requirement: Warm oneshot path

Title summary and reply-suggestion calls on the grok oneshot path SHALL reuse a warm
oneshot process when the path is already process-hot, rather than spawning a new agent
process for each call. Each oneshot call SHALL use a fresh grok ACP session (N=1) and
SHALL NOT resume a prior oneshot conversation session.

> test: code

### ~ Scenario: An oneshot call on a hot path reuses the process

- **GIVEN** a grok oneshot path that is already process-hot
- **WHEN** an oneshot call is made on that path
- **THEN** the harness does not spawn a new agent process for that call

> test: code

## + Requirement: Native Grok agent launch

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
