# @ Chat composer footer

Rules for the meta strip under the chat prompt: when the resend hint appears, how context
usage is shown by fill heat, the short closed model label, and the Missing closed label
when the effective model is not available.

## + Requirement: Missing closed model label

When the effective model for the chat is not available, the closed model control SHALL
show the label `Missing` instead of a model display name.

> test: code

### Scenario: Closed label is Missing when the effective model is not available

- **GIVEN** an effective model that is not available
- **WHEN** the closed model control label is built
- **THEN** the label is `Missing`

> test: code
