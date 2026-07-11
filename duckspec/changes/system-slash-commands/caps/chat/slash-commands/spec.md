# Chat slash commands

Kinded slash-command catalog for chat completion, local system handlers (including
`/help`), and a double-slash escape so colliding agent skills stay reachable.

## Requirement: Kinded completion catalog

Every entry in the slash completion catalog SHALL carry exactly one kind: System,
Workflow, or Agent. Duckboard system-registry names SHALL appear as System. Names
discovered from the agent harness that start with `ds-` SHALL appear as Workflow; other
discovered names SHALL appear as Agent. When the same name exists in the system registry
and in harness discovery, the catalog SHALL keep the System entry and SHALL NOT keep a
second entry for that name. The catalog SHALL NOT list Claude interactive builtins
(`clear`, `compact`, `cost`, `help`, `model`) as Agent entries from shared discovery.

> test: code

### Scenario: System registry entries are System

- **GIVEN** a system-registry command named `help`
- **WHEN** the completion catalog is built
- **THEN** the catalog includes an entry named `help`
- **AND** that entry's kind is System

> test: code

### Scenario: Discovered ds-* names are Workflow

- **GIVEN** harness discovery returns a command named `ds-spec`
- **WHEN** the completion catalog is built
- **THEN** the catalog includes an entry named `ds-spec`
- **AND** that entry's kind is Workflow

> test: code

### Scenario: Other discovered names are Agent

- **GIVEN** harness discovery returns a command named `review` that is not in the system
  registry

- **WHEN** the completion catalog is built

- **THEN** the catalog includes an entry named `review`

- **AND** that entry's kind is Agent

> test: code

### Scenario: System name wins on collision with discovery

- **GIVEN** a system-registry command named `help`
- **AND** harness discovery also returns a command named `help`
- **WHEN** the completion catalog is built
- **THEN** the catalog has exactly one entry named `help`
- **AND** that entry's kind is System

> test: code

### Scenario: Claude interactive builtins are not Agent catalog entries

- **GIVEN** a harness whose discovery uses the shared command scanner

- **WHEN** the completion catalog is built without a system override for those names

- **THEN** the catalog has no Agent entry named `clear`, `compact`, `cost`, `help`, or
  `model` that exists only as a Claude interactive builtin

> test: code

## Requirement: Local system submit

Submitting a bare system command SHALL be handled by duckboard without starting an agent
turn. For bare `/help`, the session SHALL record a user message with the submitted text
followed by a system message; the session SHALL NOT enter a streaming agent turn; pending
selection attachments SHALL remain available for a later agent turn. The system message
SHALL begin with a fixed prefix that names the system command and teaches the `//help`
escape, then list non-empty sections of the live catalog grouped by kind.

> test: code

### Scenario: Bare /help does not start an agent turn

- **GIVEN** a chat session ready to send
- **WHEN** the user submits bare `/help`
- **THEN** no agent turn is started for that submit

> test: code

### Scenario: Bare /help records user then system messages

- **GIVEN** a chat session ready to send
- **WHEN** the user submits bare `/help`
- **THEN** the transcript includes a user message whose text is `/help`
- **AND** a system message immediately after that user message

> test: code

### Scenario: Local /help leaves selection attachments intact

- **GIVEN** a chat session with a pending selection attachment
- **WHEN** the user submits bare `/help`
- **THEN** the selection attachment is still pending for a later agent turn

> test: code

### Scenario: System reply prefix names the command and teaches //help

- **GIVEN** a chat session ready to send

- **WHEN** the user submits bare `/help`

- **THEN** the system message text includes a line stating that system command `/help` is
  running

- **AND** includes guidance to use `//help` for agent help

> test: code

### Scenario: Help body lists non-empty kind sections from the live catalog

- **GIVEN** a completion catalog with at least one System entry and at least one Workflow
  entry and no Agent entries

- **WHEN** the user submits bare `/help`

- **THEN** the system message body includes a System section listing the System entries

- **AND** includes a Workflow section listing the Workflow entries

- **AND** does not include an Agent section

> test: code

## Requirement: Double-slash agent escape

A bare double-slash command (`//name`) SHALL be submitted as an agent turn whose prompt is
the single-slash form (`/name`). The user-visible message text for that submit SHALL be
the typed double-slash form.

> test: code

### Scenario: Bare //help is an agent turn with prompt /help

- **GIVEN** a chat session ready to send
- **WHEN** the user submits bare `//help`
- **THEN** an agent turn is started
- **AND** the turn prompt is `/help`

> test: code

### Scenario: Escape keeps typed //help as the user message text

- **GIVEN** a chat session ready to send
- **WHEN** the user submits bare `//help`
- **THEN** the user message text recorded for that submit is `//help`

> test: code

## Requirement: Kind cues in completion

Each completion row SHALL paint the command name token with a color determined by that
entry's kind (System, Workflow, and Agent are pairwise distinct). System rows SHALL
include a short `sys` tag. When two entries have equal fuzzy match scores, the completion
list SHALL order System before Workflow before Agent.

> test: code

### Scenario: Name token color maps by kind

- **GIVEN** completion rows for System, Workflow, and Agent entries
- **WHEN** name-token colors are resolved
- **THEN** the three kinds resolve to three different colors

> test: code

### Scenario: System rows include a sys tag

- **GIVEN** a System completion entry
- **WHEN** the completion row is rendered
- **THEN** the row includes a `sys` tag

> test: code

### Scenario: Equal fuzzy scores order System, Workflow, Agent

- **GIVEN** three catalog entries of kinds System, Workflow, and Agent that all score
  equally for the current query

- **WHEN** the filtered completion list is built

- **THEN** those three appear in order System, then Workflow, then Agent

> test: code
