# @ Chat transcript

## + Requirement: Host-choice tools omitted from Activity

Tool uses and tool results that represent mid-turn host structured questions (including
Claude's AskUserQuestion and humanized forms such as "Ask user question") SHALL NOT appear
as rows in Activity segments. Other tools in the same content stream SHALL still form
Activity rows under ordinary segment construction.

> test: code

### Scenario: AskUserQuestion tool content is omitted from Activity

- **GIVEN** a session whose assistant content includes AskUserQuestion tool use and result
  blocks and at least one ordinary tool use and result

- **WHEN** the transcript segments are built

- **THEN** the Activity segment does not include a row for the AskUserQuestion tools

- **AND** the Activity segment includes a row for the ordinary tool

> test: code
