# @ Chat transcript

## + Requirement: Meta-card line background

When an Answer segment's text is prepared for display, every line whose index falls in a
meta-card inclusive line range for that answer text SHALL receive a meta-card line
background. Lines outside those ranges SHALL NOT receive a meta-card line background.
Meta-card ranges are those produced by chat meta-card recognition for `write` and `next`
cards in the answer source. The background SHALL be visually distinct from ordinary Answer
text and from search-match and diff line backgrounds.

> test: code

### Scenario: Meta-card lines on an Answer get meta-card background

- **GIVEN** an Answer whose source text contains a recognized `next` meta card covering a
  known inclusive line range

- **WHEN** that Answer's display lines are prepared

- **THEN** every line index in that range has a meta-card line background

> test: code

### Scenario: Non-meta lines on the same Answer do not get meta-card background

- **GIVEN** an Answer whose source text has ordinary prose lines before a recognized meta
  card

- **WHEN** that Answer's display lines are prepared

- **THEN** the ordinary prose lines do not have a meta-card line background

> test: code
