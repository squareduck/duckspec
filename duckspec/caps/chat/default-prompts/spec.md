# Chat default prompts

Conversation-local empty-input defaults from a cheap-model oneshot: parse ordered `REPLY:`
suggestions (heuristic passed only as a soft hint), show and arm them only after the
oneshot settles, and drive empty Enter plus Tab cycling from that list alone.

Under-input input hints for the empty composer: when the global auto messages setting is
enabled, the effective list is always empty (obvious chrome owns lifecycle assistance).
When auto messages is disabled, an empty session seeds a single entry from the first
lifecycle option when one exists; a non-empty session uses settled agent oneshot `REPLY:`
suggestions only when the global agent input hints setting is enabled (default off). Empty
Enter and Tab cycle drive that effective list alone.

## Requirement: Parsed suggestion list

Raw model output SHALL be reduced to at most three non-empty strings taken from lines that
start with the prefix `REPLY:`, in source order. Lines that do not match that prefix SHALL
be ignored. Text after the prefix is trimmed; empty results after trim are dropped.
Unknown slash forms (including command names not in any allow-list) SHALL be kept as
written. A soft character budget in the oneshot instruction SHALL NOT cause the parser to
truncate reply text — over-budget strings SHALL be kept in full after trim.

> test: code

### Scenario: REPLY lines extracted in order and capped at three

- **GIVEN** model output with four lines each starting with `REPLY:` and non-empty text
- **WHEN** the suggestion list is parsed
- **THEN** the list has exactly three entries
- **AND** the entries are the first three reply texts in source order

> test: code
> - crates/duckchat/src/reply_suggest.rs:148

### Scenario: No matching lines yields an empty list

- **GIVEN** model output with no line starting with `REPLY:`
- **WHEN** the suggestion list is parsed
- **THEN** the list is empty

> test: code
> - crates/duckchat/src/reply_suggest.rs:161

### Scenario: Unknown slash text is preserved

- **GIVEN** model output with a `REPLY:` line whose text is an unknown slash form
- **WHEN** the suggestion list is parsed
- **THEN** the list contains that slash text unchanged

> test: code
> - crates/duckchat/src/reply_suggest.rs:168

### Scenario: Reply longer than 100 characters is preserved in full

- **GIVEN** model output with a `REPLY:` line whose text after trim is longer than 100
  characters

- **WHEN** the suggestion list is parsed

- **THEN** the list contains that full reply text unchanged (no character truncation)

> test: code
> - crates/duckchat/src/reply_suggest.rs:234

## Requirement: Oneshot request framing

The reply-suggestion request SHALL carry the last assistant message, the preceding user
message when present, the project's discovered slash command names as priming hints, and
the lifecycle heuristic when the session has one. The heuristic SHALL be a soft hint only
— the oneshot MAY omit it, place it in any position, or invent other replies. When
embedding message bodies into the oneshot prompt, the assistant message SHALL be limited
to its last 40 lines and the user message SHALL be limited to its last 12 lines; when a
body is truncated, a truncation marker SHALL appear so earlier content is not implied to
be present. The instruction framing SHALL require 1–3 lines of the form `REPLY: <text>`
with this order when multiple lines are emitted: first line the most obvious continuation
of the flow; any middle lines alternatives; last line a negative or declining option when
a negative option is appropriate. The instruction framing SHALL soft-ask that each REPLY
text be at most 100 characters; that budget SHALL NOT be enforced by truncating parsed
results. An empty assistant message SHALL yield an empty suggestion list without calling
the model.

### Scenario: Heuristic is included in the request when present

- **GIVEN** a lifecycle heuristic for the session
- **WHEN** the reply-suggestion request is built
- **THEN** the request includes that heuristic as a soft hint

> test: code
> - crates/duckchat/src/reply_suggest.rs:181

### Scenario: Ordering guidance is present in the instruction

- **GIVEN** the shared reply-suggestion instruction text

- **WHEN** the instruction is inspected

- **THEN** it requires first-line obvious continue, middle alternatives, and last-line
  negative when appropriate

> test: code
> - crates/duckchat/src/reply_suggest.rs:193

### Scenario: Empty assistant yields empty list without a model call

- **GIVEN** an empty assistant message
- **WHEN** reply suggestions are requested
- **THEN** the suggestion list is empty
- **AND** no model call is made

> test: code
> - crates/duckchat/src/reply_suggest.rs:276

### Scenario: Long assistant message is truncated to its last lines

- **GIVEN** a last assistant message longer than 40 lines
- **WHEN** the reply-suggestion request prompt body is built
- **THEN** the embedded assistant body keeps only the last 40 lines
- **AND** a truncation marker is present

> test: code
> - crates/duckchat/src/reply_suggest.rs:288

### Scenario: Long user message is truncated to its last lines

- **GIVEN** a preceding user message longer than 12 lines
- **WHEN** the reply-suggestion request prompt body is built
- **THEN** the embedded user body keeps only the last 12 lines
- **AND** a truncation marker is present

> test: code
> - crates/duckchat/src/reply_suggest.rs:328

### Scenario: Length guidance is present in the instruction

- **GIVEN** the shared reply-suggestion instruction text
- **WHEN** the instruction is inspected
- **THEN** it soft-asks that each REPLY text be at most 100 characters

> test: code
> - crates/duckchat/src/reply_suggest.rs:219

## Requirement: Effective default-prompt list

The effective default-prompt list is built as follows. When auto messages is enabled, the
effective list SHALL be empty regardless of session emptiness, lifecycle options, agent
input hints, or oneshot storage — under-input hints are fully suppressed so obvious chrome
owns lifecycle assistance. When auto messages is disabled and the session transcript is
empty and a first lifecycle option is present, the effective list SHALL be exactly that
option in empty-send form (a single entry); the list SHALL NOT wait on a oneshot and SHALL
NOT include oneshot parse results. When auto messages is disabled and the session
transcript is empty and no first lifecycle option is present, the effective list SHALL be
empty. When auto messages is disabled and the session transcript is non-empty and agent
input hints are enabled, and a reply-suggestion oneshot has settled for the current
generation with one or more parsed reply strings, the effective list SHALL be exactly those
strings (order preserved, already capped); the first lifecycle option SHALL NOT be appended
or merged into that list. When auto messages is disabled and the session transcript is
non-empty and agent input hints are enabled, and no such non-empty oneshot result is armed
— including a settled oneshot that failed or produced no suggestions — the effective list
SHALL be empty, whether or not a first lifecycle option is present. When auto messages is
disabled and the session transcript is non-empty and agent input hints are disabled, the
effective list SHALL be empty regardless of oneshot storage or lifecycle options. The first
lifecycle option SHALL NOT appear as an effective-list entry for a non-empty session.

> test: code

### Scenario: Parsed replies are the effective list in order

- **GIVEN** a non-empty session transcript
- **AND** agent input hints enabled
- **AND** auto messages disabled
- **AND** a settled oneshot whose parse produced three distinct reply strings
- **WHEN** the effective default-prompt list is built
- **THEN** the list is exactly those three strings in parse order

> test: code
> - crates/duckboard/src/default_prompts.rs:214

### Scenario: No non-empty oneshot result yields an empty list

- **GIVEN** a non-empty session transcript
- **AND** agent input hints enabled
- **AND** auto messages disabled
- **AND** no settled non-empty oneshot result
- **WHEN** the effective default-prompt list is built
- **THEN** the list is empty

> test: code
> - crates/duckboard/src/default_prompts.rs:230

### Scenario: Failed or empty oneshot yields an empty list even with a heuristic

- **GIVEN** a non-empty session transcript
- **AND** agent input hints enabled
- **AND** auto messages disabled
- **AND** a settled oneshot that failed or produced no suggestions
- **AND** a present first lifecycle option
- **WHEN** the effective default-prompt list is built
- **THEN** the list is empty

> test: code
> - crates/duckboard/src/default_prompts.rs:238

### Scenario: Empty session seeds first lifecycle

- **GIVEN** an empty session transcript
- **AND** a first lifecycle option in empty-send form
- **AND** auto messages disabled
- **WHEN** the effective default-prompt list is built
- **THEN** the list is exactly that single lifecycle option

> test: code
> - crates/duckboard/src/default_prompts.rs:254

### Scenario: Empty session without lifecycle yields empty

- **GIVEN** an empty session transcript
- **AND** no first lifecycle option
- **AND** auto messages disabled
- **WHEN** the effective default-prompt list is built
- **THEN** the list is empty

> test: code
> - crates/duckboard/src/default_prompts.rs:268

### Scenario: Non-empty session with agent hints disabled yields empty despite oneshot

- **GIVEN** a non-empty session transcript
- **AND** agent input hints disabled
- **AND** auto messages disabled
- **AND** a settled oneshot whose parse produced one or more reply strings
- **WHEN** the effective default-prompt list is built
- **THEN** the list is empty

> test: code
> - crates/duckboard/src/default_prompts.rs:277

### Scenario: Empty session ignores oneshot results

- **GIVEN** an empty session transcript
- **AND** a first lifecycle option in empty-send form
- **AND** auto messages disabled
- **AND** stored oneshot reply strings that differ from that option
- **WHEN** the effective default-prompt list is built
- **THEN** the list is exactly that single lifecycle option

> test: code
> - crates/duckboard/src/default_prompts.rs:285

### Scenario: Auto messages on yields empty even for empty session seed

- **GIVEN** an empty session transcript
- **AND** a first lifecycle option in empty-send form
- **AND** auto messages enabled
- **WHEN** the effective default-prompt list is built
- **THEN** the list is empty

> test: code
> - crates/duckboard/src/default_prompts.rs:293

## Requirement: Suggestion readiness

After a non-priming turn completes and a reply-suggestion oneshot is started for a
generation, suggestions SHALL be pending until that oneshot settles (success or failure
for any reason, including oneshot timeout) for the same generation. While pending and the
composer input is empty, the default-prompt list SHALL NOT be presented and a loading
indicator SHALL be shown instead; empty submit SHALL NOT send a default prompt; Tab and
Shift-Tab SHALL NOT cycle defaults. When the oneshot settles for the matching generation,
suggestions SHALL become ready: the effective list is presented when non-empty and
empty-input send and cycle are armed. Results for a superseded generation SHALL NOT
present or arm suggestions. When no oneshot is outstanding, suggestions are ready (the
effective list may be empty). If the chat agent handle ends while a reply-suggestion
oneshot is outstanding for the current generation without a matching settle, suggestions
SHALL become ready (they SHALL NOT remain pending). While a main agent turn is in progress
(streaming), empty-input default prompts SHALL NOT be presented — neither as a list nor as
a loading indicator — regardless of oneshot readiness or whether a non-empty effective
list would otherwise be available.

### Scenario: Pending hides list and shows loading

- **GIVEN** a pending reply-suggestion oneshot
- **AND** an empty composer input
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** the default-prompt list is not shown
- **AND** a loading indicator is shown

> test: code
> - crates/duckboard/src/default_prompts.rs:380

### Scenario: Empty submit is a no-op while pending

- **GIVEN** a pending reply-suggestion oneshot
- **AND** an empty composer input
- **WHEN** the user submits
- **THEN** no message is sent

> test: code
> - crates/duckboard/src/default_prompts.rs:347

### Scenario: Ready after settle arms the effective list

- **GIVEN** a reply-suggestion oneshot that has settled for the current generation
- **AND** a non-empty effective default-prompt list
- **AND** an empty composer input
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** the effective list is shown
- **AND** empty submit sends the active entry

> test: code
> - crates/duckboard/src/default_prompts.rs:355

### Scenario: Superseded generation does not arm the list

- **GIVEN** a oneshot result whose generation no longer matches the session
- **WHEN** that result is applied
- **THEN** the session's ready default-prompt list is unchanged

> test: code
> - crates/duckboard/src/default_prompts.rs:373

### Scenario: Main turn in progress hides default prompts

- **GIVEN** a main agent turn is in progress
- **AND** an empty composer input
- **AND** a non-empty effective default-prompt list would otherwise be available
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** the default-prompt list is not shown
- **AND** a defaults loading indicator is not shown

> test: code
> - crates/duckboard/src/default_prompts.rs:409

### Scenario: Timed-out or failed oneshot settles to ready

- **GIVEN** a pending reply-suggestion oneshot
- **AND** that oneshot settles as a failure for the current generation
- **AND** an empty composer input
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** a loading indicator is not shown
- **AND** suggestions are ready
- **AND** the effective list is empty when the failure produced no parse

> test: code
> - crates/duckboard/src/default_prompts.rs:428

### Scenario: Agent handle ends while suggestions pending

- **GIVEN** a pending reply-suggestion oneshot
- **AND** the chat agent handle ends without a settle for that generation
- **AND** an empty composer input
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** a loading indicator is not shown
- **AND** suggestions are ready

> test: code
> - crates/duckboard/src/default_prompts.rs:455

## Requirement: Empty-input send and cycle

When the composer input is empty, suggestions are ready, and the effective default-prompt
list is non-empty, submit SHALL send the active entry of that list. When the list is empty
or suggestions are not ready, empty submit SHALL NOT send. Tab and Shift-Tab SHALL advance
and reverse the active index with wrap when the input is empty, suggestions are ready, the
list is non-empty, and slash-command completion is not consuming Tab. Cycling the active
entry SHALL NOT insert text into the composer input.

> test: code

### Scenario: Empty submit sends the active prompt

- **GIVEN** an empty composer input
- **AND** ready suggestions
- **AND** a non-empty effective default-prompt list
- **AND** an active index into that list
- **WHEN** the user submits
- **THEN** the sent text is the entry at the active index

> test: code
> - crates/duckboard/src/default_prompts.rs:317

### Scenario: Empty submit is a no-op when the list is empty

- **GIVEN** an empty composer input
- **AND** ready suggestions
- **AND** an empty effective default-prompt list
- **WHEN** the user submits
- **THEN** no message is sent

> test: code
> - crates/duckboard/src/default_prompts.rs:328

### Scenario: Tab cycles active index with wrap

- **GIVEN** an empty composer input
- **AND** ready suggestions
- **AND** an effective default-prompt list of at least two entries
- **AND** slash-command completion is not consuming Tab
- **WHEN** the user presses Tab at the last index
- **THEN** the active index wraps to the first entry
- **AND** the composer input remains empty

> test: code
> - crates/duckboard/src/default_prompts.rs:335

## Requirement: Defaults list presentation

When the empty-input defaults chrome presents the ready effective list, each suggestion
SHALL soft-wrap within the composer width. Each list row's height SHALL follow its wrapped
content so consecutive suggestion rows do not overlap. The full suggestion text SHALL
remain visible — the chrome SHALL NOT hard-truncate or ellipsize the displayed value for
length.

### Scenario: Long suggestion soft-wraps without overlapping the next row

- **GIVEN** a ready non-empty effective default-prompt list
- **AND** an empty composer input
- **AND** at least one suggestion whose text is wider than the composer pane
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** that suggestion's text soft-wraps within the composer width
- **AND** the following suggestion row does not overlap the wrapped text

> manual: iced layout — confirm no paint-through between consecutive default rows

### Scenario: Full suggestion text is visible for a multi-line row

- **GIVEN** a ready non-empty effective default-prompt list
- **AND** an empty composer input
- **AND** a suggestion that wraps to more than one visual line
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** the entire suggestion text is visible without ellipsis or hard clip

> manual: visual check that multi-line default rows show full text

## Requirement: Agent input hints gate

A global agent input hints setting SHALL control whether reply-suggestion oneshots run
after turns, and a global auto messages setting SHALL suppress oneshots entirely when
enabled (chrome owns assistance). The agent input hints setting SHALL default to disabled;
auto messages SHALL default to enabled. When agent input hints is disabled, or when auto
messages is enabled, a reply-suggestion oneshot SHALL NOT be started after a non-priming
turn completes. When agent input hints is enabled and auto messages is disabled, oneshot
launch follows the existing non-priming turn rules (assistant text present, and other
launch conditions in this capability).

> test: code

### Scenario: Default agent input hints setting is disabled

- **GIVEN** application config defaults
- **WHEN** the agent input hints setting is read
- **THEN** it is disabled

> test: code
> - crates/duckboard/src/config.rs:246

### Scenario: Oneshot launch requires agent input hints enabled

- **GIVEN** agent input hints disabled
- **AND** auto messages disabled
- **AND** a non-priming turn that would otherwise qualify for reply suggestions
- **WHEN** oneshot launch is decided
- **THEN** a reply-suggestion oneshot is not started

> test: code
> - crates/duckboard/src/default_prompts.rs:305
