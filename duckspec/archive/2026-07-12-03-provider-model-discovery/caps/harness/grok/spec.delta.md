# @ Grok harness

## @ Requirement: Model discovery

Listing models SHALL return grok's available models, each tagged with the grok harness,
carrying a human-readable display name, and carrying its own context window. Title-summary
and reply-suggestion oneshots SHALL select the preferred oneshot model for the grok
harness when that model is among the available models, and SHALL fall back to another
available model when the preferred model is absent.

> test: code

### ~ Scenario: Title model falls back when the preferred fast model is absent

- **GIVEN** a set of available models that does not include the preferred oneshot model
  for the grok harness

- **WHEN** the harness selects a model for title summarisation or reply suggestion

- **THEN** it selects another available model rather than failing

> test: code

### + Scenario: Each listed model carries a display name

- **GIVEN** a grok handshake advertising its available models
- **WHEN** the harness lists models
- **THEN** each returned model carries a non-empty display name

> test: code
