# @ OpenAI Codex harness

## @ Requirement: Profile-compatible event emission

The Codex ACP agent SHALL emit profile `session/update` notifications so the shared ACP
client can map them to neutral agent events: assistant text as content updates, a tool
invocation as a tool-use update followed by a completed result update sharing the same
call id, and token telemetry as a usage update. Token telemetry SHALL carry the latest
turn's total-token count when present, SHALL use cumulative thread total only when
latest-turn usage is unavailable, and SHALL emit no usage update when neither total
exists. While a turn is in progress, the agent SHALL deliver those profile updates to the
host as they become available, rather than only after the turn has completed.

### ~ Scenario: Token telemetry surfaces as usage with total tokens

- **GIVEN** Codex reporting both latest-turn and cumulative token usage
- **AND** the cumulative total is larger than the latest-turn total
- **WHEN** the agent translates that telemetry for the ACP client
- **THEN** it emits a usage update carrying the latest-turn total

> test: code

### + Scenario: Cumulative token telemetry is used when latest-turn usage is absent

- **GIVEN** Codex reporting cumulative token usage without latest-turn usage
- **WHEN** the agent translates that telemetry for the ACP client
- **THEN** it emits a usage update carrying the cumulative total

> test: code

### + Scenario: Missing token totals emit no usage update

- **GIVEN** a Codex token-usage notification with neither a latest-turn nor cumulative
  total

- **WHEN** the agent translates that telemetry for the ACP client

- **THEN** it emits no usage update

> test: code

## + Requirement: Repository-scoped VCS access

Every Codex turn SHALL use a workspace-write sandbox policy whose additional writable
roots are the existing `.git` and `.jj` directories directly beneath the normalized
repository root supplied by ACP session open or load. The agent SHALL refresh that
repository context on session open or load, retain it independently of app-server process
heat, and apply it to every turn. It SHALL NOT add absent metadata, file indirections,
ancestor metadata, or external stores. If the backend rejects the policy, the turn SHALL
fail through the app-server error path without retrying under a weaker or broader policy.

> test: code

### Scenario: Direct repository metadata is writable on every turn

- **GIVEN** a normalized repository root with direct `.git` and `.jj` directories
- **WHEN** the Codex agent starts a turn for that repository
- **THEN** the turn uses workspace-write
- **AND** its additional writable roots contain those direct metadata directories
- **AND** the policy is supplied on every turn

> test: code

### Scenario: External metadata indirection is not granted

- **GIVEN** a repository root whose `.git` entry is a file that points to an external
  store

- **WHEN** the Codex agent derives the turn's additional writable roots

- **THEN** it does not follow or grant access to that external store

- **AND** it grants no writable root for the `.git` file

> test: code

### Scenario: Resumed and restarted sessions reapply refreshed repository access

- **GIVEN** a persisted Codex thread whose repository metadata has changed since its
  previous turn

- **AND** the app-server process has restarted

- **WHEN** the ACP client loads the session and starts its next turn

- **THEN** the agent refreshes repository access from the load working directory

- **AND** the resumed turn receives the refreshed workspace-write policy

> test: code

### Scenario: A rejected repository policy does not trigger a weaker retry

- **GIVEN** the app-server rejects a turn's repository workspace-write policy
- **WHEN** the Codex agent handles that rejection
- **THEN** the turn fails through the app-server error path
- **AND** the agent does not retry the turn with missing, weaker, or broader permissions

> test: code
