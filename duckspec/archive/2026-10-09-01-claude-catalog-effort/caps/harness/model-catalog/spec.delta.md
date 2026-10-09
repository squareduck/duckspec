# ~ Harness model catalog

The process catalog refreshes every harness at app start. Grok and Codex clear on an empty
or failed rediscovery. Claude keeps the last good slice across a failed refresh, and is
refreshed again at catalog expiry and while that slice is stale or empty.

## ~ Requirement: Startup catalog refresh

At app start the process SHALL refresh the model catalog for each available provider from
that provider’s discovery path. A successful refresh for a harness SHALL replace that
harness’s catalog slice with the discovered models.

> test: code

### Scenario: App start refreshes models for each available provider

- **GIVEN** more than one registered provider that can offer models

- **WHEN** the app starts and the model catalog is refreshed

- **THEN** each available provider’s discovery path is used to populate that harness’s
  catalog slice

> test: code
> - crates/duckboard/src/agent.rs:517

### Scenario: Successful refresh replaces that harness’s catalog slice

- **GIVEN** a harness with a prior catalog slice
- **AND** a successful rediscovery that yields a different non-empty model set
- **WHEN** the catalog is refreshed for that harness
- **THEN** the harness’s catalog slice is the newly discovered set

> test: code
> - crates/duckboard/src/agent.rs:536

## ~ Requirement: Catalog is the selection source

The models offered for selection SHALL be the contents of the process model catalog.
Looking up the context window for a selected model SHALL use the catalog entry for that
model’s harness and id when present.

> test: code

### Scenario: Offered selectable models are the catalog contents

- **GIVEN** a process model catalog with models from one or more harnesses
- **WHEN** the selectable models are listed
- **THEN** the listed models are exactly the catalog contents

> test: code
> - crates/duckboard/src/agent.rs:590

### Scenario: Context window lookup uses the catalog entry for the selected model

- **GIVEN** a catalog entry for a model with a known context window
- **AND** that model selected
- **WHEN** the context window for the selected model is resolved
- **THEN** the resolved window is the window from that catalog entry

> test: code
> - crates/duckboard/src/agent.rs:613

## ~ Requirement: Clear slice on empty rediscovery

A harness without a last-good policy clears its slice when rediscovery is empty or fails.

> test: code

### Scenario: Empty rediscovery clears the prior harness list

- **GIVEN** a harness that does not keep a last-good catalog
- **AND** that harness’s catalog slice is non-empty
- **AND** a rediscovery for that harness that yields an empty set
- **WHEN** the catalog is refreshed for that harness
- **THEN** the harness’s catalog slice is empty

> test: code
> - crates/duckboard/src/agent.rs:560

### Scenario: Cold failure leaves that harness empty without panic

- **GIVEN** a harness that does not keep a last-good catalog
- **AND** that harness has no prior successful discovery
- **AND** discovery for that harness failing or yielding an empty set
- **WHEN** the catalog is refreshed for that harness
- **THEN** the harness’s catalog slice is empty
- **AND** the refresh completes without panicking

> test: code
> - crates/duckboard/src/agent.rs:575

## + Requirement: Claude catalog retention

A failed Claude refresh keeps the last good catalog. A successful one replaces it, and a
successful empty catalog stays empty.

> test: code

### Scenario: Failed Claude refresh keeps the last good catalog

- **GIVEN** a Claude catalog refresh that fails or reports the catalog failed

- **WHEN** the process catalog applies that result

- **THEN** a non-empty provider memo is kept

- **AND** otherwise a slice already applied this process is kept

- **AND** otherwise the snapshot at `~/.config/duckboard/claude-catalog.json` is installed
  as stored, including effort when the stored row has it, under the `claude-code` harness,
  even past its expiry

- **AND** otherwise the Claude slice stays empty

- **AND** a missing or unreadable snapshot file installs nothing

> test: code

### Scenario: Successful Claude refresh replaces the catalog

- **GIVEN** a successful Claude catalog with models

- **AND** a successful Claude catalog with no models and a snapshot file already on disk

- **WHEN** the process catalog applies each result

- **THEN** the catalog with models replaces the Claude slice, the provider memo, and the
  snapshot

- **AND** the empty catalog clears the Claude slice and the provider memo and leaves the
  snapshot file in place

- **AND** a later failed refresh after that empty success leaves the Claude slice empty

> test: code

## + Requirement: Claude refresh schedule

After app start, only Claude is refreshed again, and only while its catalog is due.

> test: code

### Scenario: Claude is refreshed again only when its catalog is due

- **GIVEN** a fresh Claude catalog with an expiry
- **AND** a fresh Claude catalog with no expiry
- **AND** a Claude slice that is empty or past its expiry
- **WHEN** the refresh schedule is evaluated
- **THEN** the fresh catalog with an expiry arms one wake at that expiry and does not poll
- **AND** the empty or past-expiry catalog retries every five minutes
- **AND** the catalog with no expiry arms neither wake nor retry until the next launch
- **AND** the wake and the retry refresh Claude only

> test: code
