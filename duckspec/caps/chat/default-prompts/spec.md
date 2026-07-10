# Chat default prompts

Conversation-local empty-input defaults from a cheap-model oneshot: parse ordered `REPLY:`
suggestions (heuristic passed only as a soft hint), show and arm them only after the
oneshot settles, and drive empty Enter plus Tab cycling from that list alone.

Conversation-local empty-input defaults: the lifecycle heuristic arms the list before any
oneshot and after a failed or empty oneshot; a settled oneshot with parsed replies
replaces the list. Show and arm after readiness rules; drive empty Enter plus Tab cycling
from the effective list.

## Requirement: Parsed suggestion list

Raw model output SHALL be reduced to at most three non-empty strings taken from lines that
start with the prefix `REPLY:`, in source order. Lines that do not match that prefix SHALL
be ignored. Text after the prefix is trimmed; empty results after trim are dropped.
Unknown slash forms (including command names not in any allow-list) SHALL be kept as
written.

> test: code

### Scenario: REPLY lines extracted in order and capped at three

- **GIVEN** model output with four lines each starting with `REPLY:` and non-empty text
- **WHEN** the suggestion list is parsed
- **THEN** the list has exactly three entries
- **AND** the entries are the first three reply texts in source order

> test: code
> - crates/duckchat/src/reply_suggest.rs:142

### Scenario: No matching lines yields an empty list

- **GIVEN** model output with no line starting with `REPLY:`
- **WHEN** the suggestion list is parsed
- **THEN** the list is empty

> test: code
> - crates/duckchat/src/reply_suggest.rs:155

### Scenario: Unknown slash text is preserved

- **GIVEN** model output with a `REPLY:` line whose text is an unknown slash form
- **WHEN** the suggestion list is parsed
- **THEN** the list contains that slash text unchanged

> test: code
> - crates/duckchat/src/reply_suggest.rs:162

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
a negative option is appropriate. An empty assistant message SHALL yield an empty
suggestion list without calling the model.

### Scenario: Heuristic is included in the request when present

- **GIVEN** a lifecycle heuristic for the session
- **WHEN** the reply-suggestion request is built
- **THEN** the request includes that heuristic as a soft hint

> test: code
> - crates/duckchat/src/reply_suggest.rs:175

### Scenario: Ordering guidance is present in the instruction

- **GIVEN** the shared reply-suggestion instruction text

- **WHEN** the instruction is inspected

- **THEN** it requires first-line obvious continue, middle alternatives, and last-line
  negative when appropriate

> test: code
> - crates/duckchat/src/reply_suggest.rs:187

### Scenario: Empty assistant yields empty list without a model call

- **GIVEN** an empty assistant message
- **WHEN** reply suggestions are requested
- **THEN** the suggestion list is empty
- **AND** no model call is made

> test: code
> - crates/duckchat/src/reply_suggest.rs:245

### Scenario: Long assistant message is truncated to its last lines

- **GIVEN** a last assistant message longer than 40 lines
- **WHEN** the reply-suggestion request prompt body is built
- **THEN** the embedded assistant body keeps only the last 40 lines
- **AND** a truncation marker is present

> test: code
> - crates/duckchat/src/reply_suggest.rs:257

### Scenario: Long user message is truncated to its last lines

- **GIVEN** a preceding user message longer than 12 lines
- **WHEN** the reply-suggestion request prompt body is built
- **THEN** the embedded user body keeps only the last 12 lines
- **AND** a truncation marker is present

> test: code
> - crates/duckchat/src/reply_suggest.rs:297

## Requirement: Effective default-prompt list

The effective default-prompt list is built as follows. When a reply-suggestion oneshot has
settled for the current generation with one or more parsed reply strings, the effective
list SHALL be exactly those strings (order preserved, already capped); the lifecycle
heuristic SHALL NOT be appended or merged into that list. When no such non-empty oneshot
result is armed — including a brand-new session that has never run a oneshot, and a
settled oneshot that failed or produced no suggestions — and a lifecycle heuristic is
present, the effective list SHALL be a single entry: that heuristic in empty-send form
(leading `/` when the heuristic is a skill name such as `ds-explore`). When neither a
non-empty oneshot result nor a heuristic is available, the effective list SHALL be empty.
Pre-oneshot heuristic defaults SHALL be ready without a model call (no pending oneshot
required).

> test: code

### Scenario: Parsed replies are the effective list in order

- **GIVEN** a settled oneshot whose parse produced three distinct reply strings
- **WHEN** the effective default-prompt list is built
- **THEN** the list is exactly those three strings in parse order

> test: code
> - crates/duckboard/src/default_prompts.rs:177

### Scenario: Pre-oneshot list is the lifecycle heuristic when present

- **GIVEN** a session with no settled non-empty oneshot result
- **AND** a present lifecycle heuristic
- **WHEN** the effective default-prompt list is built
- **THEN** the list has exactly one entry
- **AND** that entry is the heuristic in empty-send form

> test: code
> - crates/duckboard/src/default_prompts.rs:192

### Scenario: Failed or empty oneshot falls back to the heuristic

- **GIVEN** a settled oneshot that failed or produced no suggestions
- **AND** a present lifecycle heuristic
- **WHEN** the effective default-prompt list is built
- **THEN** the list has exactly one entry
- **AND** that entry is the heuristic in empty-send form

> test: code
> - crates/duckboard/src/default_prompts.rs:200

### Scenario: No oneshot and no heuristic yields an empty list

- **GIVEN** a session with no settled non-empty oneshot result
- **AND** no lifecycle heuristic
- **WHEN** the effective default-prompt list is built
- **THEN** the list is empty

> test: code
> - crates/duckboard/src/default_prompts.rs:214

## Requirement: Suggestion readiness

After a non-priming turn completes and a reply-suggestion oneshot is started for a
generation, suggestions SHALL be pending until that oneshot settles (success or failure)
for the same generation. While pending and the composer input is empty, the default-prompt
list SHALL NOT be presented and a loading indicator SHALL be shown instead; empty submit
SHALL NOT send a default prompt; Tab and Shift-Tab SHALL NOT cycle defaults. When the
oneshot settles for the matching generation, suggestions SHALL become ready: the effective
list is presented when non-empty and empty-input send and cycle are armed. Results for a
superseded generation SHALL NOT present or arm suggestions. When no oneshot is
outstanding, suggestions are ready (the effective list may be empty). While a main agent
turn is in progress (streaming), empty-input default prompts SHALL NOT be presented —
neither as a list nor as a loading indicator — regardless of oneshot readiness or whether
a non-empty effective list would otherwise be available.

### Scenario: Pending hides list and shows loading

- **GIVEN** a pending reply-suggestion oneshot
- **AND** an empty composer input
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** the default-prompt list is not shown
- **AND** a loading indicator is shown

> test: code
> - crates/duckboard/src/default_prompts.rs:290

### Scenario: Empty submit is a no-op while pending

- **GIVEN** a pending reply-suggestion oneshot
- **AND** an empty composer input
- **WHEN** the user submits
- **THEN** no message is sent

> test: code
> - crates/duckboard/src/default_prompts.rs:251

### Scenario: Ready after settle arms the effective list

- **GIVEN** a reply-suggestion oneshot that has settled for the current generation
- **AND** a non-empty effective default-prompt list
- **AND** an empty composer input
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** the effective list is shown
- **AND** empty submit sends the active entry

> test: code
> - crates/duckboard/src/default_prompts.rs:259

### Scenario: Superseded generation does not arm the list

- **GIVEN** a oneshot result whose generation no longer matches the session
- **WHEN** that result is applied
- **THEN** the session's ready default-prompt list is unchanged

> test: code
> - crates/duckboard/src/default_prompts.rs:278

### Scenario: Main turn in progress hides default prompts

- **GIVEN** a main agent turn is in progress
- **AND** an empty composer input
- **AND** a non-empty effective default-prompt list would otherwise be available
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** the default-prompt list is not shown
- **AND** a defaults loading indicator is not shown

> test: code
> - crates/duckboard/src/default_prompts.rs:319

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
> - crates/duckboard/src/default_prompts.rs:221

### Scenario: Empty submit is a no-op when the list is empty

- **GIVEN** an empty composer input
- **AND** ready suggestions
- **AND** an empty effective default-prompt list
- **WHEN** the user submits
- **THEN** no message is sent

> test: code
> - crates/duckboard/src/default_prompts.rs:232

### Scenario: Tab cycles active index with wrap

- **GIVEN** an empty composer input
- **AND** ready suggestions
- **AND** an effective default-prompt list of at least two entries
- **AND** slash-command completion is not consuming Tab
- **WHEN** the user presses Tab at the last index
- **THEN** the active index wraps to the first entry
- **AND** the composer input remains empty

> test: code
> - crates/duckboard/src/default_prompts.rs:239
