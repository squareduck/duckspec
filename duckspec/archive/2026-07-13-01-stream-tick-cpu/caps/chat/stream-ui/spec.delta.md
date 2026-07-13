# @ Chat stream UI

## + Requirement: Stream UI tick need

A session SHALL need the stream UI tick while a turn is streaming and the agent is not
awaiting a mid-turn user choice (so animation and pure-content materialize can run on the
tick cadence). A session that is only streaming while awaiting a user choice SHALL need
the stream UI tick only when pure-content dirtiness is owed on the tick under
stick-to-bottom (deferred materialize). Idle mid-turn await with no such deferred
materialize SHALL NOT need the stream UI tick.

> test: code

### Scenario: Active streaming without awaiting needs the stream UI tick

- **GIVEN** a streaming turn
- **AND** the session is not awaiting a mid-turn user choice
- **WHEN** stream UI tick need is evaluated for that session
- **THEN** the session needs the stream UI tick

> test: code

### Scenario: Idle awaiting without deferred materialize does not need the stream UI tick

- **GIVEN** a streaming turn that is awaiting a mid-turn user choice
- **AND** pure-content dirtiness is not owed for stick-to-bottom materialize on the tick
- **WHEN** stream UI tick need is evaluated for that session
- **THEN** the session does not need the stream UI tick

> test: code

### Scenario: Awaiting with deferred pure content on stick-to-bottom needs the stream UI tick

- **GIVEN** a streaming turn that is awaiting a mid-turn user choice
- **AND** pure-content dirtiness is owed for stick-to-bottom materialize on the tick
- **WHEN** stream UI tick need is evaluated for that session
- **THEN** the session needs the stream UI tick

> test: code
