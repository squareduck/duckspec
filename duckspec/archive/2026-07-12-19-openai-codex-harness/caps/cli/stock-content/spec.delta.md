# @ Stock CLI content

## @ Requirement: Stock content from the binary

### + Scenario: Known codex skills are installed under .agents/skills

- **GIVEN** a supported harness `codex` whose stock stage skills are carried in the binary

- **AND** a working directory with no `.agents/skills` tree yet

- **WHEN** `ds init codex` is run in that directory

- **THEN** `.agents/skills/` contains skill directories for the stock duckspec stages

- **AND** each skill directory includes a `SKILL.md` whose body matches the stock skill
  body from the binary

- **AND** the command exits successfully

> test: code
