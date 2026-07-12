# Chat answer landmarks

Full-width contrast on the latest Answer and keyboard jumps between Answer starts and chat
history ends, so long transcripts stay easy to re-enter.

## Requirement: Last Answer contrast band

The transcript SHALL treat exactly one Answer as the band target when at least one Answer
has non-empty body text: the latest such Answer in transcript order. That Answer SHALL be
presented with a full-width background that is more contrasty than the ordinary chat
surface and SHALL NOT use card chrome (no distinct bubble border or card-style side
inset). Older Answers and Answers with empty body text SHALL NOT be band targets.
Thinking, Activity, User, and System segments SHALL NOT receive this band.

> test: code

### Scenario: Sole latest non-empty Answer is the band target

- **GIVEN** a transcript with more than one Answer segment that has non-empty body text
- **WHEN** the last-Answer band target is resolved
- **THEN** only the latest non-empty Answer is the band target
- **AND** every earlier Answer is not a band target

> test: code

### Scenario: Empty latest Answer is not a band target

- **GIVEN** a transcript whose latest Answer segment has empty body text
- **AND** an earlier Answer segment has non-empty body text
- **WHEN** the last-Answer band target is resolved
- **THEN** the empty latest Answer is not the band target
- **AND** the latest non-empty Answer is the band target

> test: code

## Requirement: Answer reply anchors

Reply navigation anchors SHALL be the Answer segments of the transcript in order.
Thinking, Activity, User, and System segments SHALL NOT be reply anchors. From a current
Answer anchor, previous SHALL resolve to the immediately preceding Answer anchor when one
exists, and next SHALL resolve to the immediately following Answer anchor when one exists.
At the first Answer, previous SHALL yield no target. At the last Answer, next SHALL yield
no target. Navigation SHALL NOT wrap.

> test: code

### Scenario: Only Answer blocks are reply anchors

- **GIVEN** a transcript that mixes Answer segments with Thinking, Activity, or User
  segments

- **WHEN** the reply-anchor list is built

- **THEN** the anchors are exactly the Answer segments in transcript order

- **AND** no Thinking, Activity, or User segment is an anchor

> test: code

### Scenario: Prev and next step to adjacent Answer anchors

- **GIVEN** a transcript with at least three Answer anchors
- **AND** the current Answer is the middle of those three
- **WHEN** previous and next reply targets are resolved
- **THEN** previous is the Answer immediately before the current one
- **AND** next is the Answer immediately after the current one

> test: code

### Scenario: Prev at first and next at last yield no target

- **GIVEN** a transcript with at least one Answer anchor

- **WHEN** previous is resolved from the first Answer and next is resolved from the last
  Answer

- **THEN** there is no previous target

- **AND** there is no next target

> test: code

## Requirement: Viewport current for reply jumps

When the chat is stuck to the bottom, the current Answer for previous/next resolution
SHALL be the last Answer anchor. Otherwise the current Answer SHALL be the last Answer
anchor whose top is at or above the viewport top (scroll offset); if none qualify, the
current Answer SHALL be the first Answer anchor. When there are no Answer anchors, there
SHALL be no current Answer.

> test: code

### Scenario: Stick-to-bottom treats the last Answer as current

- **GIVEN** a transcript with more than one Answer anchor
- **AND** the chat is stuck to the bottom
- **WHEN** the current Answer for reply jumps is resolved
- **THEN** the current Answer is the last Answer anchor

> test: code

### Scenario: Scroll offset selects the Answer at or above the viewport top

- **GIVEN** a transcript with more than one Answer anchor with known tops
- **AND** the chat is not stuck to the bottom
- **AND** the viewport top lies at or below one Answer top and above the next
- **WHEN** the current Answer for reply jumps is resolved
- **THEN** the current Answer is the last Answer whose top is at or above the viewport top

> test: code

## Requirement: History end jumps

History-top navigation SHALL move the chat transcript viewport to the start of the chat
history. History-bottom navigation SHALL move the viewport to the end of the chat history
and leave the chat stuck to the bottom so further growth continues to follow the end.
History-top and previous/next reply jumps SHALL clear stick-to-bottom so the viewport does
not snap back to the end while the user is reading.

## Requirement: Landmark shortcut eligibility

Landmark shortcuts (history top/bottom and previous/next Answer) SHALL be available when
the chat tab is the active interaction with a session, including while the composer is
focused. Bare arrow keys SHALL remain available to the focused composer for caret
movement. When a modal that owns navigation keys is open, landmark shortcuts SHALL NOT
claim those keys.
