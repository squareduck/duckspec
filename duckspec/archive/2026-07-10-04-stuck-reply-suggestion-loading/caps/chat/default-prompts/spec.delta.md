# @ Chat default prompts

## @ Requirement: Suggestion readiness

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

### + Scenario: Timed-out or failed oneshot settles to ready

- **GIVEN** a pending reply-suggestion oneshot
- **AND** that oneshot settles as a failure for the current generation
- **AND** a present lifecycle heuristic
- **AND** an empty composer input
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** a loading indicator is not shown
- **AND** the effective list is the heuristic in empty-send form

> test: code

### + Scenario: Agent handle ends while suggestions pending

- **GIVEN** a pending reply-suggestion oneshot
- **AND** the chat agent handle ends without a settle for that generation
- **AND** an empty composer input
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** a loading indicator is not shown
- **AND** suggestions are ready

> test: code
