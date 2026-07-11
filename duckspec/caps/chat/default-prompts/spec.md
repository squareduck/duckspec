# Chat default prompts

Conversation-local empty-input defaults from a cheap-model oneshot: parse ordered `REPLY:`
suggestions (heuristic passed only as a soft hint), show and arm them only after the
oneshot settles, and drive empty Enter plus Tab cycling from that list alone.

Empty-composer next actions from lifecycle bootstrap (empty session) or a trailing `next`
meta card (after the first turn), shown as ghost text with empty Enter and Tab cycle;
optional settings-gated oneshot reply suggestion as a single under-input line sent only
with empty Cmd-Enter.

## Requirement: Parsed suggestion list

Raw model output SHALL be reduced to at most one non-empty string taken from lines that
start with the prefix `REPLY:`, in source order. Lines that do not match that prefix SHALL
be ignored. Text after the prefix is trimmed; empty results after trim are dropped.
Unknown slash forms (including command names not in any allow-list) SHALL be kept as
written. A soft character budget in the oneshot instruction SHALL NOT cause the parser to
truncate reply text — over-budget strings SHALL be kept in full after trim. When more than
one `REPLY:` line is present, only the first non-empty result SHALL be kept.

> test: code

### Scenario: REPLY lines capped at one

- **GIVEN** model output with two lines each starting with `REPLY:` and non-empty text
- **WHEN** the suggestion list is parsed
- **THEN** the list has exactly one entry
- **AND** that entry is the first reply text in source order

> test: code
> - crates/duckchat/src/reply_suggest.rs:64

### Scenario: No matching lines yields an empty list

- **GIVEN** model output with no line starting with `REPLY:`
- **WHEN** the suggestion list is parsed
- **THEN** the list is empty

> test: code
> - crates/duckchat/src/reply_suggest.rs:78

### Scenario: Unknown slash text is preserved

- **GIVEN** model output with a `REPLY:` line whose text is an unknown slash form
- **WHEN** the suggestion list is parsed
- **THEN** the list contains that slash text unchanged

> test: code
> - crates/duckchat/src/reply_suggest.rs:85

### Scenario: Reply longer than 100 characters is preserved in full

- **GIVEN** model output with a `REPLY:` line whose text after trim is longer than 100
  characters

- **WHEN** the suggestion list is parsed

- **THEN** the list contains that full reply text unchanged (no character truncation)

> test: code
> - crates/duckchat/src/reply_suggest.rs:98

## Requirement: Oneshot request framing

The reply-suggestion request SHALL carry the full last assistant message and the preceding
user message when present, without line-count truncation and without a truncation marker
for omitted earlier lines. The request SHALL NOT include a lifecycle heuristic. The
request SHALL NOT include discovered slash command names as priming hints. The instruction
framing SHALL ask for at most one line of the form `REPLY: <text>` that suggests a natural
freeform user response continuing the dialogue from those messages, and SHALL NOT prefer
duckspec stage slash commands as the default job of the oneshot. An empty assistant
message SHALL yield an empty suggestion list without calling the model.

> test: code

### Scenario: Full assistant and user messages are embedded without line truncation

- **GIVEN** a last assistant message longer than 40 lines
- **AND** a preceding user message longer than 12 lines
- **WHEN** the reply-suggestion request prompt body is built
- **THEN** the embedded assistant body includes the full assistant message
- **AND** the embedded user body includes the full user message
- **AND** no line-truncation marker is present for omitted earlier content

> test: code
> - crates/duckchat/src/reply_suggest.rs:108

### Scenario: Lifecycle heuristic is not included in the request

- **GIVEN** a session that has a first lifecycle option
- **WHEN** the reply-suggestion request prompt body is built
- **THEN** the prompt body does not include a lifecycle heuristic block

> test: code
> - crates/duckchat/src/reply_suggest.rs:155

### Scenario: Instruction asks for a freeform user reply and at most one REPLY line

- **GIVEN** the shared reply-suggestion instruction text
- **WHEN** the instruction is inspected
- **THEN** it asks for a natural freeform user reply continuing the dialogue
- **AND** it allows at most one `REPLY:` line
- **AND** it does not prefer stage slash commands as the oneshot's primary job

> test: code
> - crates/duckchat/src/reply_suggest.rs:172

### Scenario: Empty assistant yields empty list without a model call

- **GIVEN** an empty assistant message
- **WHEN** reply suggestions are requested
- **THEN** the suggestion list is empty
- **AND** no model call is made

> test: code
> - crates/duckchat/src/reply_suggest.rs:199

## Requirement: Oneshot readiness

After a non-priming turn completes and a reply-suggestion oneshot is started for a
generation, oneshot suggestions SHALL be pending until that oneshot settles (success or
failure for any reason, including oneshot timeout) for the same generation. While pending
and the composer input is empty, the under-input oneshot row SHALL NOT present a
suggestion string and a loading indicator SHALL be shown instead; empty Cmd-Enter SHALL
NOT send a oneshot suggestion. Pending oneshot state SHALL NOT block empty Enter from
sending an armed next action. When the oneshot settles for the matching generation,
oneshot suggestions SHALL become ready: a non-empty single suggestion is presented under
the input and empty Cmd-Enter is armed. Results for a superseded generation SHALL NOT
present or arm oneshot suggestions. When no oneshot is outstanding, oneshot suggestions
are ready (the suggestion may be absent). If the chat agent handle ends while a
reply-suggestion oneshot is outstanding for the current generation without a matching
settle, oneshot suggestions SHALL become ready (they SHALL NOT remain pending). While a
main agent turn is in progress (streaming), under-input oneshot chrome SHALL NOT be
presented — neither as a suggestion row nor as a loading indicator.

> test: code

### Scenario: Pending hides oneshot row and shows loading

- **GIVEN** a pending reply-suggestion oneshot
- **AND** an empty composer input
- **WHEN** the under-input oneshot chrome is rendered
- **THEN** the oneshot suggestion string is not shown
- **AND** a loading indicator is shown

> test: code
> - crates/duckboard/src/default_prompts.rs:482

### Scenario: Empty Cmd-Enter is a no-op while oneshot pending

- **GIVEN** a pending reply-suggestion oneshot
- **AND** an empty composer input
- **WHEN** the user presses Cmd-Enter
- **THEN** no oneshot suggestion is sent

> test: code
> - crates/duckboard/src/default_prompts.rs:497

### Scenario: Empty Enter still sends next action while oneshot pending

- **GIVEN** a pending reply-suggestion oneshot
- **AND** an empty composer input
- **AND** a non-empty next-action list with an active entry
- **WHEN** the user submits with empty input
- **THEN** the sent text is that active next-action entry

> test: code
> - crates/duckboard/src/default_prompts.rs:443

### Scenario: Ready after settle arms the oneshot row

- **GIVEN** a reply-suggestion oneshot that has settled for the current generation
- **AND** a non-empty oneshot suggestion
- **AND** an empty composer input
- **WHEN** the under-input oneshot chrome is rendered
- **THEN** that suggestion is shown
- **AND** empty Cmd-Enter sends that suggestion

> test: code
> - crates/duckboard/src/default_prompts.rs:508

### Scenario: Superseded generation does not arm oneshot

- **GIVEN** a oneshot result whose generation no longer matches the session
- **WHEN** that result is applied
- **THEN** the session's ready oneshot suggestion is unchanged

> test: code
> - crates/duckboard/src/default_prompts.rs:525

### Scenario: Main turn in progress hides oneshot chrome

- **GIVEN** a main agent turn is in progress
- **AND** an empty composer input
- **AND** a non-empty oneshot suggestion would otherwise be available
- **WHEN** the under-input oneshot chrome is rendered
- **THEN** the oneshot suggestion is not shown
- **AND** a oneshot loading indicator is not shown

> test: code
> - crates/duckboard/src/default_prompts.rs:532

### Scenario: Timed-out or failed oneshot settles to ready empty

- **GIVEN** a pending reply-suggestion oneshot
- **AND** that oneshot settles as a failure for the current generation
- **AND** an empty composer input
- **WHEN** the under-input oneshot chrome is rendered
- **THEN** a loading indicator is not shown
- **AND** oneshot suggestions are ready
- **AND** no oneshot suggestion string is shown when the failure produced no parse

> test: code
> - crates/duckboard/src/default_prompts.rs:551

### Scenario: Agent handle ends while oneshot pending becomes ready

- **GIVEN** a pending reply-suggestion oneshot
- **AND** the chat agent handle ends without a settle for that generation
- **AND** an empty composer input
- **WHEN** the under-input oneshot chrome is rendered
- **THEN** a loading indicator is not shown
- **AND** oneshot suggestions are ready

> test: code
> - crates/duckboard/src/default_prompts.rs:572

## Requirement: Agent input hints gate

A global agent input hints setting SHALL control whether reply-suggestion oneshots run
after turns. The setting SHALL default to disabled. When agent input hints is disabled, a
reply-suggestion oneshot SHALL NOT be started after a non-priming turn completes. When
agent input hints is enabled, oneshot launch follows the non-priming turn rules of this
capability (assistant text present and other launch conditions). There is no separate
auto-messages setting that suppresses oneshots or next-action lists. Empty-session
next-action bootstrap and the next-action list SHALL NOT depend on the agent input hints
setting.

### Scenario: Default agent input hints setting is disabled

- **GIVEN** application config defaults
- **WHEN** the agent input hints setting is read
- **THEN** it is disabled

> test: code
> - crates/duckboard/src/config.rs:235

### Scenario: Oneshot launch requires agent input hints enabled

- **GIVEN** agent input hints disabled
- **AND** a non-priming turn that would otherwise qualify for reply suggestions
- **WHEN** oneshot launch is decided
- **THEN** a reply-suggestion oneshot is not started

> test: code
> - crates/duckboard/src/default_prompts.rs:470

### Scenario: Empty-session next actions remain when agent input hints disabled

- **GIVEN** agent input hints disabled
- **AND** an empty session transcript
- **AND** a first lifecycle option for that session
- **WHEN** the next-action list is built
- **THEN** the list is exactly that single lifecycle option in empty-send form

> test: code
> - crates/duckboard/src/area/interaction.rs:742

## Requirement: Next-action list

The next-action list for the empty composer SHALL be built as follows. When the session
transcript is empty and a first lifecycle option is present, the list SHALL be exactly
that option in empty-send form (a single entry). When the session transcript is empty and
no first lifecycle option is present, the list SHALL be empty. When the session transcript
is non-empty, the list SHALL be exactly the trailing next actions extracted from the last
non-priming assistant message (via chat meta-card recognition); if that message has no
trailing `next` meta card, the list SHALL be empty. Settled oneshot suggestion strings
SHALL NOT be appended, merged, or substituted into the next-action list. Disk lifecycle
options beyond the empty-session bootstrap SHALL NOT fill the list after the first turn.

For an empty exploration session, the first lifecycle option SHALL be the explore stage
command. For an empty change session, the first lifecycle option SHALL be the first option
of that change's lifecycle ladder from its artifact and step state. Sessions with no
lifecycle ladder (including caps and codex) SHALL have no first lifecycle option from this
bootstrap.

### Scenario: Empty session seeds first lifecycle

- **GIVEN** an empty session transcript
- **AND** a first lifecycle option in empty-send form
- **WHEN** the next-action list is built
- **THEN** the list is exactly that single lifecycle option

> test: code
> - crates/duckboard/src/default_prompts.rs:275

### Scenario: Empty session without lifecycle yields empty

- **GIVEN** an empty session transcript
- **AND** no first lifecycle option
- **WHEN** the next-action list is built
- **THEN** the list is empty

> test: code
> - crates/duckboard/src/default_prompts.rs:288

### Scenario: Non-empty session uses trailing next actions only

- **GIVEN** a non-empty session transcript

- **AND** a last non-priming assistant message whose trailing next actions are two
  distinct send tokens

- **AND** a first lifecycle option that differs from those tokens

- **WHEN** the next-action list is built

- **THEN** the list is exactly those two trailing next send tokens in order

> test: code
> - crates/duckboard/src/default_prompts.rs:298

### Scenario: Non-empty session without trailing next yields empty

- **GIVEN** a non-empty session transcript
- **AND** a last non-priming assistant message with no trailing `next` meta card
- **AND** a present first lifecycle option
- **WHEN** the next-action list is built
- **THEN** the list is empty

> test: code
> - crates/duckboard/src/default_prompts.rs:318

### Scenario: Oneshot results do not enter the next-action list

- **GIVEN** a non-empty session transcript
- **AND** a last non-priming assistant message with no trailing `next` meta card
- **AND** a settled oneshot whose parse produced a non-empty suggestion
- **WHEN** the next-action list is built
- **THEN** the list is empty

> test: code
> - crates/duckboard/src/default_prompts.rs:327

### Scenario: Empty exploration session seeds explore

- **GIVEN** an empty exploration session transcript
- **WHEN** the next-action list is built
- **THEN** the list is exactly the explore stage command in empty-send form

> test: code
> - crates/duckboard/src/area/interaction.rs:708

### Scenario: Empty change session with unfinished steps seeds apply

- **GIVEN** an empty change session transcript
- **AND** that change has at least one unfinished step
- **WHEN** the next-action list is built
- **THEN** the list is exactly the apply stage command in empty-send form

> test: code
> - crates/duckboard/src/area/interaction.rs:721

## Requirement: Next-action empty-input send and cycle

When the composer input is empty and the next-action list is non-empty, submit SHALL send
the active entry's send text. When the next-action list is empty, empty submit SHALL NOT
send a next action. Tab and Shift-Tab SHALL advance and reverse the active next-action
index with wrap when the input is empty, the list has at least two entries, and
slash-command completion is not consuming Tab. Cycling SHALL NOT insert text into the
composer input. When the next-action list has more than one entry and the input is empty,
a tab-available marker SHALL be shown before the ghost (next-action) affordance. When the
list has zero or one entry, that tab-available marker SHALL NOT be shown. The empty
composer's ghost (placeholder) text SHALL be the active next-action send text when the
list is non-empty (and the main turn is not streaming).

> test: code

### Scenario: Empty submit sends the active next action

- **GIVEN** an empty composer input
- **AND** a non-empty next-action list
- **AND** an active index into that list
- **WHEN** the user submits
- **THEN** the sent text is the send text of the entry at the active index

> test: code
> - crates/duckboard/src/default_prompts.rs:342

### Scenario: Empty submit is a no-op when the next-action list is empty

- **GIVEN** an empty composer input
- **AND** an empty next-action list
- **WHEN** the user submits
- **THEN** no next-action message is sent

> test: code
> - crates/duckboard/src/default_prompts.rs:364

### Scenario: Tab cycles next actions with wrap

- **GIVEN** an empty composer input
- **AND** a next-action list of at least two entries
- **AND** slash-command completion is not consuming Tab
- **WHEN** the user presses Tab at the last index
- **THEN** the active index wraps to the first entry
- **AND** the composer input remains empty

> test: code
> - crates/duckboard/src/default_prompts.rs:390

### Scenario: Multi next shows a tab-available marker

- **GIVEN** an empty composer input
- **AND** a next-action list of at least two entries
- **WHEN** the empty-composer next-action chrome is rendered
- **THEN** a tab-available marker is shown before the ghost text

> test: code
> - crates/duckboard/src/default_prompts.rs:416

## Requirement: Oneshot empty-input send

When the composer input is empty, oneshot suggestions are ready, and a non-empty oneshot
suggestion is armed, Cmd-Enter SHALL send that suggestion. When no oneshot suggestion is
armed, or oneshot suggestions are not ready, empty Cmd-Enter SHALL NOT send a oneshot
suggestion. Empty Enter SHALL NOT send the oneshot suggestion (next actions own empty
Enter). Empty Shift-Enter SHALL NOT send the oneshot suggestion.

> test: code

### Scenario: Empty Cmd-Enter sends the armed oneshot suggestion

- **GIVEN** an empty composer input
- **AND** ready oneshot suggestions
- **AND** a non-empty armed oneshot suggestion
- **WHEN** the user presses Cmd-Enter
- **THEN** the sent text is that oneshot suggestion

> test: code
> - crates/duckboard/src/default_prompts.rs:589

### Scenario: Empty Cmd-Enter is a no-op when no oneshot suggestion

- **GIVEN** an empty composer input
- **AND** ready oneshot suggestions
- **AND** no armed oneshot suggestion
- **WHEN** the user presses Cmd-Enter
- **THEN** no oneshot suggestion is sent

> test: code
> - crates/duckboard/src/default_prompts.rs:599

### Scenario: Empty Enter does not send the oneshot suggestion

- **GIVEN** an empty composer input
- **AND** ready oneshot suggestions
- **AND** a non-empty armed oneshot suggestion
- **AND** an empty next-action list
- **WHEN** the user submits with empty input
- **THEN** no message is sent

> test: code
> - crates/duckboard/src/default_prompts.rs:614

## Requirement: Oneshot presentation

When the under-input oneshot chrome presents a ready non-empty oneshot suggestion, a
Cmd-Enter marker SHALL appear before the suggestion text. The marker SHALL be legible in
the UI (not a broken fallback glyph). The suggestion SHALL soft-wrap within the composer
width. The full suggestion text SHALL remain visible — the chrome SHALL NOT hard-truncate
or ellipsize the displayed value for length.

> test: code

### Scenario: Armed oneshot shows a Cmd-Enter marker before the suggestion

- **GIVEN** a ready non-empty oneshot suggestion
- **AND** an empty composer input
- **WHEN** the under-input oneshot chrome is rendered
- **THEN** a Cmd-Enter marker is shown before the suggestion text

> test: code
> - crates/duckboard/src/default_prompts.rs:627

### Scenario: Long oneshot soft-wraps without clipping

- **GIVEN** a ready non-empty oneshot suggestion whose text is wider than the composer
  pane

- **AND** an empty composer input

- **WHEN** the under-input oneshot chrome is rendered

- **THEN** that suggestion's text soft-wraps within the composer width

- **AND** the entire suggestion text is visible without ellipsis or hard clip

> manual: iced layout — confirm oneshot row soft-wraps and shows full text
