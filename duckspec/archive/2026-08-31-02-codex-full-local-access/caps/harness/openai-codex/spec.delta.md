# @ OpenAI Codex harness

## = Requirement: Repository-scoped VCS access

Requirement: Full local access

## ~ Requirement: Full local access

Every Codex turn uses an explicit unrestricted local-access policy under the permissions
of Duckboard's OS user, without inheriting or falling back to another access boundary.

> test: code

### Scenario: Every turn receives full local access

- **GIVEN** a Codex session whose app-server process remains hot
- **WHEN** the agent runs consecutive turns on that session
- **THEN** each turn receives the explicit full-local-access policy

### Scenario: A resumed session reapplies full local access after restart

- **GIVEN** a persisted Codex thread whose app-server process has restarted
- **WHEN** the agent resumes the thread and starts its next turn
- **THEN** the resumed turn receives the explicit full-local-access policy

### Scenario: A rejected full-access policy does not trigger a fallback

- **GIVEN** the app-server rejects a turn's full-local-access policy
- **WHEN** the Codex agent handles that rejection
- **THEN** the turn fails through the app-server error path
- **AND** the agent does not retry with a missing, inherited, or sandboxed policy
