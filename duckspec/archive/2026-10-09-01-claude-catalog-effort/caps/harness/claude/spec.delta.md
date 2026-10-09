# @ Claude harness

The Claude harness drives Claude Code through an owned ACP agent child over the official
`claude` CLI: the official process starts on the first user prompt (not at open), sessions
bind Claude's native ids for resume, the main path keeps a duplex-hot Claude process after
that bind while the prompt's model and effort still match, the agent advertises the Claude
Code catalog on initialize, and the agent streams profile-compatible `session/update`
notifications (text, tools, and thinking) for the shared ACP client.

## ~ Requirement: Model discovery

The host's Claude model list is the set the agent advertised on initialize, carried
through with the same name, window, and effort.

> test: code

### Scenario: Listed models follow the agent advertise set

- **GIVEN** the owned Claude agent advertising two models on initialize
- **AND** one model has a display name, a context window, and an effort scale
- **AND** the other model has a display name and neither a window nor an effort scale
- **WHEN** the harness lists models
- **THEN** the listed models are exactly that advertised set
- **AND** each listed model is tagged with the Claude harness
- **AND** the first model carries that display name, context window, and effort scale
- **AND** the second model carries its display name and no context window or effort scale

> test: code

## ~ Requirement: Agent model advertise

Initialize advertises the current Claude Code catalog's selectable rows and reports
whether that fetch succeeded.

> test: code

### Scenario: Admitted catalog rows carry name, window, and effort

- **GIVEN** a Claude Code catalog and a `claude` binary whose leading version triple
  satisfies one main row and is older than another main row's minimum

- **AND** the satisfied main row has a display name, a context window, and an effort scale

- **AND** another main row the version satisfies has no effort scale

- **AND** the catalog also contains a row outside the main section

- **WHEN** the agent completes initialize

- **THEN** the satisfied row is advertised with that display name, that context window,
  and that effort scale's levels in catalog order plus its default level

- **AND** the row with no effort scale is advertised without effort

- **AND** the too-new row and the non-main row are not advertised

> test: code

### Scenario: An unreadable binary version drops rows that require a minimum

- **GIVEN** a `claude` binary whose version cannot be read as a leading `N.N.N`

- **AND** a catalog with a main row that requires a minimum version and a main row that
  does not

- **WHEN** the agent completes initialize

- **THEN** the row without a minimum is advertised

- **AND** the row that requires a minimum is not advertised

> test: code

### Scenario: A failed catalog fetch advertises no models

- **GIVEN** a catalog fetch that fails
- **WHEN** the agent completes initialize
- **THEN** initialize succeeds
- **AND** the catalog is reported failed
- **AND** no models are advertised

> test: code

### Scenario: Catalog expiry is reported only when the document has one

- **GIVEN** one successful catalog whose document includes an expiry time
- **AND** another successful catalog whose document includes none
- **WHEN** the agent completes initialize for each catalog
- **THEN** both report the catalog ok
- **AND** only the document that has an expiry time is reported with that expiry

> test: code

## @ Requirement: Duplex main heat

The main path reuses the inner `claude` process only while it is still that session and
was spawned with this prompt's model and effort.

> test: code

### - Scenario: A second main turn reuses the inner Claude process when duplex-hot

### + Scenario: Hot process follows the prompt's model and effort

- **GIVEN** a duplex-hot Claude process for a native session, spawned with a model and an
  effort level

- **WHEN** a later main prompt matches that spawn, another changes the model or effort,
  and another has no effort level

- **THEN** the matching prompt reuses the hot process

- **AND** a changed model or effort kills that process and resumes the same native session
  id with `--model` for the prompt

- **AND** `--effort` is passed for a prompt that has a level and omitted for a prompt that
  has none

> test: code

### + Scenario: A model or effort change during an in-flight turn does not cancel that turn

- **GIVEN** an in-flight Claude main turn
- **WHEN** the chat's model or effort changes
- **THEN** that turn is not cancelled

> test: code

### + Scenario: A title or reply oneshot leaves the main hot process in place

- **GIVEN** a duplex-hot Claude main process
- **WHEN** a title-summary or reply-suggestion oneshot runs
- **THEN** that main hot process is still in place
- **AND** the oneshot does not pass an effort flag

> test: code
