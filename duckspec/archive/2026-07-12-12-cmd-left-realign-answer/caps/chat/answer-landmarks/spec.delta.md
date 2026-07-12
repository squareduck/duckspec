# @ Chat answer landmarks

## @ Requirement: Answer reply anchors

Reply navigation anchors SHALL be the Answer segments of the transcript in order.
Thinking, Activity, User, and System segments SHALL NOT be reply anchors. From a current
Answer anchor, next SHALL resolve to the immediately following Answer anchor when one
exists. When the viewport is already at the top of the current Answer, previous SHALL
resolve to the immediately preceding Answer anchor when one exists. At the first Answer
with the viewport at its top, previous SHALL yield no target. At the last Answer, next
SHALL yield no target. Navigation SHALL NOT wrap.

> test: code

## + Requirement: Previous reply re-align

When resolving the previous reply navigation target, if the viewport top is strictly below
the top of the current Answer (beyond the same alignment slack used to select the current
Answer from scroll offset), the target SHALL be that current Answer. Otherwise previous
SHALL follow the prior-Answer rule from Answer reply anchors. Next reply navigation SHALL
NOT re-align to the current Answer; it SHALL always use the next Answer anchor rule.

> test: code

### Scenario: Viewport below current top targets current Answer

- **GIVEN** a transcript with more than one Answer anchor with known tops
- **AND** a resolved current Answer
- **AND** the viewport top is strictly below that Answer's top
- **WHEN** the previous reply target is resolved
- **THEN** the target is the current Answer

> test: code

### Scenario: At current top previous targets prior Answer

- **GIVEN** a transcript with more than one Answer anchor with known tops
- **AND** a resolved current Answer that is not the first
- **AND** the viewport top is at that Answer's top
- **WHEN** the previous reply target is resolved
- **THEN** the target is the Answer immediately before the current one

> test: code

### Scenario: Next ignores re-align when below current top

- **GIVEN** a transcript with more than one Answer anchor with known tops
- **AND** a resolved current Answer that is not the last
- **AND** the viewport top is strictly below that Answer's top
- **WHEN** the next reply target is resolved
- **THEN** the target is the Answer immediately after the current one

> test: code
