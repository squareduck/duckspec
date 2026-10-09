# @ ACP client

The shared ACP client runtime drives every harness turn: it spawns a launch-parameterized
agent child, opens or resumes a session, maps profile `session/update` notifications into
neutral agent events, and keeps the main agent process warm across turns until cancel.
Initialize may carry optional per-model effort and catalog status, and a turn's effort
level is copied onto `session/prompt` as `effort`.

## + Requirement: Initialize handshake metadata

Optional per-model effort and an optional catalog status ride on initialize. A handshake
that omits them stays a normal model list.

> test: code

### Scenario: Advertised effort is carried when the model sends it

- **GIVEN** an initialize handshake with two models
- **AND** one model includes an effort object with a default and a list of levels
- **AND** the other model includes no effort object
- **WHEN** the client parses the handshake
- **THEN** the first model carries that default and those levels
- **AND** the second model carries no effort

> test: code

### Scenario: Catalog status is optional handshake metadata

- **GIVEN** a handshake with no catalog object
- **AND** a handshake whose catalog object is ok and includes an expiry
- **AND** a handshake whose catalog object is ok and includes no expiry
- **AND** a handshake whose catalog object is failed
- **WHEN** the client parses each handshake
- **THEN** the handshake with no catalog object reads as ok with no expiry
- **AND** the ok object that includes an expiry keeps ok and that expiry
- **AND** the ok object with no expiry keeps ok and no expiry
- **AND** the failed object keeps failed

> test: code

## + Requirement: Prompt effort

A turn's effort level is a `session/prompt` field of its own, separate from the
reasoning-mode field.

> test: code

### Scenario: Effort and reasoning mode use different prompt fields

- **GIVEN** a turn request that carries an effort level and no reasoning mode

- **AND** a turn request that carries a reasoning mode and no effort level

- **AND** a turn request that carries neither

- **WHEN** the client builds `session/prompt` for each

- **THEN** the effort-level turn sends that level as `effort` and leaves `reasoningEffort`
  unset

- **AND** the reasoning-mode turn sends `reasoningEffort` and leaves `effort` unset

- **AND** the turn with neither omits both

> test: code
