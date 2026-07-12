# @ Harness model catalog

Process-local catalog of models from each available provider: refreshed once at app start,
cleared for a harness when rediscovery is empty or fails, and used as the source for
selectable models and context-window lookup.

## - Requirement: Keep last good on empty rediscovery

## + Requirement: Clear slice on empty rediscovery

When rediscovery for a harness yields an empty set or fails, the catalog SHALL clear that
harness’s catalog slice (including any previous non-empty list). When a harness has never
had a successful discovery, an empty or failed rediscovery SHALL leave that harness’s
slice empty without panicking.

> test: code

### Scenario: Empty rediscovery clears the prior harness list

- **GIVEN** a harness whose catalog slice is non-empty
- **AND** a rediscovery for that harness that yields an empty set
- **WHEN** the catalog is refreshed for that harness
- **THEN** the harness’s catalog slice is empty

> test: code

### Scenario: Cold failure leaves that harness empty without panic

- **GIVEN** a harness with no prior successful discovery
- **AND** discovery for that harness failing or yielding an empty set
- **WHEN** the catalog is refreshed for that harness
- **THEN** the harness’s catalog slice is empty
- **AND** the refresh completes without panicking

> test: code
