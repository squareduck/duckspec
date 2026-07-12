# @ Chat fast response

Source-neutral option chips for mid-turn structured choices and settled oneshot reply
suggestions: ordered options with ⌘-number activation, ephemeral view layout, and
empty-send formatting for bare skill names. A live user-choice request fills the shell for
in-band answers and shows the question as a chip above the options when prompt text is
present. Settling a choice (pick or freeform) commits host question and answer transcript
blocks; cancel commits neither. Freeform composer submit while awaiting completes the
pending choice as a custom answer. While awaiting, the composer section uses a quiet
accent tint, including the model selector.

## @ Requirement: Ephemeral chips

Option chips and other pre-settle fast-response chrome SHALL NOT be stored in the session
transcript as committed messages while only shown as view chrome. That rule does not
forbid host question and answer transcript blocks committed when a user choice settles.

> test: code

### ~ Scenario: Visible chips are not a stored user message

- **GIVEN** fast-response option chips are shown

- **AND** no chip action has been activated

- **WHEN** the session transcript is inspected

- **THEN** it does not contain a committed user-choice answer for those chips

- **AND** it does not contain a user text message whose sole purpose is a fast-response
  option chip

> test: code

## @ Requirement: Question activation

When the shell is filled from a pending mid-turn user choice and chips are visible,
activating an option SHALL complete that choice with the selected option on the agent wire
(in-band). It SHALL NOT send a new ordinary user turn for that activation. It SHALL commit
the host settled choice transcript for that answer (question when present, then answer).

> test: code

### - Scenario: Option activation answers the pending choice without a new user message

### + Scenario: Option activation answers in-band and commits host question and answer

- **GIVEN** visible chips filled from a pending user choice with a non-empty question and
  at least one option

- **WHEN** the first option is activated

- **THEN** the pending choice is completed with that option on the agent wire

- **AND** the session transcript gains a host question block with that question text

- **AND** the session transcript gains a host answer block with that option's label and no
  hotkey prefix

- **AND** the session does not gain a new ordinary user text turn solely for the
  activation

> test: code

## @ Requirement: Freeform while awaiting

When the session is awaiting a user choice and the user submits non-empty freeform
composer text, the system SHALL complete that pending choice as a **custom answer** whose
payload is that freeform text (so the harness finishes the question tool with free text as
the answer value, not as cancel/skip). It SHALL clear the choice shell. It SHALL NOT leave
the text only staged in the interrupt queue, and SHALL NOT send a new ordinary user turn
solely for that custom answer. It SHALL commit the host settled choice transcript for that
answer (question when present, then answer with the freeform text).

> test: code

### ~ Scenario: Freeform submit completes the pending choice as a custom answer

- **GIVEN** a session awaiting a user choice with non-empty fast-response options

- **AND** a non-empty question on that choice

- **AND** non-empty freeform text in the composer

- **WHEN** the user submits the composer

- **THEN** the pending choice is completed as a custom answer carrying that freeform text

- **AND** the session transcript gains a host question block with that question text

- **AND** the session transcript gains a host answer block with that freeform text

- **AND** the session does not gain a new ordinary user text turn solely for that custom
  answer

- **AND** the text is not left only staged in the interrupt queue

> test: code

## + Requirement: Live question chip

While the session is awaiting a user choice and the shell is filled from that choice, when
the choice has non-empty question text the system SHALL present that text as a question
chip above the option chips. The presented question label SHALL use the form
`Question: <text>`, prepending `Question: ` when the text does not already begin with that
prefix. The question chip SHALL NOT be a numbered selectable option. When the choice has
empty or missing question text, the system SHALL omit the question chip and still present
the option chips under ordinary visibility rules.

> test: code

### Scenario: Non-empty prompt shows a question chip above options

- **GIVEN** a session awaiting a user choice
- **AND** non-empty question text on that choice
- **AND** non-empty option chips for that choice
- **WHEN** the live fast-response chrome is presented
- **THEN** a question chip appears above the option chips
- **AND** the question chip label begins with `Question: `
- **AND** the question chip is not a numbered selectable option

> test: code

### Scenario: Empty prompt omits the question chip

- **GIVEN** a session awaiting a user choice
- **AND** empty or missing question text on that choice
- **AND** non-empty option chips for that choice
- **WHEN** the live fast-response chrome is presented
- **THEN** no question chip is shown
- **AND** the option chips are still shown under ordinary visibility rules

> test: code

## + Requirement: Settled choice transcript

When a pending mid-turn user choice settles with an answer (selected option label or
freeform text), the session SHALL commit host transcript content for that exchange: a
question entry when the choice had non-empty question text, then an answer entry whose
text is the answer without a hotkey prefix. The stored question entry body SHALL use the
form `Question: <text>`, prepending `Question: ` when the source text does not already
begin with that prefix. When the choice had empty or missing question text, the session
SHALL commit the answer entry only. When the choice is cancelled, the session SHALL NOT
commit question or answer host entries for that choice.

> test: code

### Scenario: Settle with a prompt commits question then answer without a hotkey

- **GIVEN** a pending user choice with non-empty question text

- **AND** a settled answer string for that choice

- **WHEN** the choice is settled

- **THEN** the session transcript includes a host question entry whose body begins with
  `Question: `

- **AND** the session transcript includes a host answer entry with that answer string and
  no hotkey prefix

- **AND** the question entry appears before the answer entry

> test: code

### Scenario: Settle without a prompt commits answer only

- **GIVEN** a pending user choice with empty or missing question text
- **AND** a settled answer string for that choice
- **WHEN** the choice is settled
- **THEN** the session transcript includes a host answer entry with that answer string
- **AND** the session transcript does not include a host question entry for that choice

> test: code

### Scenario: Cancel commits no choice blocks

- **GIVEN** a pending user choice with non-empty question text
- **WHEN** the choice is cancelled
- **THEN** the session transcript does not gain a host question entry for that choice
- **AND** the session transcript does not gain a host answer entry for that choice

> test: code
