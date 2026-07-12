# @ Claude harness

## + Requirement: Oneshot preferred model

Title-summary and reply-suggestion oneshots on the Claude harness SHALL select the
preferred cheap/fast model (the curated `haiku` alias) when that model is among the models
the agent advertises. When the preferred model is not advertised, those oneshots SHALL
select another advertised model rather than failing. Main conversation turns SHALL NOT be
required to use this preferred oneshot model (session model selection is separate).

> test: code

### Scenario: Preferred oneshot model is selected when advertised

- **GIVEN** the Claude agent advertising available models that include the preferred
  oneshot model among others

- **WHEN** the harness selects a model for a title-summary or reply-suggestion oneshot

- **THEN** it selects the preferred oneshot model

> test: code

### Scenario: Oneshot model falls back when preferred is absent

- **GIVEN** the Claude agent advertising available models that do not include the
  preferred oneshot model

- **WHEN** the harness selects a model for a title-summary or reply-suggestion oneshot

- **THEN** it selects another advertised model rather than failing

> test: code
