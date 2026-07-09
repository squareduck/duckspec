# @ Grok harness

## + Requirement: Prompt attachments

When assembling a turn for `session/prompt`, the harness SHALL walk the folded prompt text
for markdown links of the form `[label](attach:<id>)`, resolve each link against the
turn's attachments map, and send a multi-block ACP `prompt` array. A resolved image
attachment SHALL appear as an ACP image content block carrying that attachment's media
type and payload. Surrounding text SHALL appear as text content blocks. A resolved
non-image attachment SHALL appear as a text content block rather than an image block. An
unresolved `attach:` link SHALL be left as its original literal markdown text.

> test: code

### Scenario: A resolved image attachment is sent as an ACP image block

- **GIVEN** a turn whose prompt contains an `attach:` link
- **AND** the turn's attachments map holds an image payload for that link's id
- **WHEN** the harness assembles the prompt for the turn
- **THEN** the `session/prompt` content includes an image content block
- **AND** that block carries the attachment's media type and payload

> test: code

### Scenario: Surrounding text is preserved as text blocks

- **GIVEN** a prompt with text before and after a resolved image `attach:` marker

- **WHEN** the harness assembles the prompt for the turn

- **THEN** the text before the marker appears as a text content block before the image
  block

- **AND** the text after the marker appears as a text content block after the image block

> test: code

### Scenario: A non-image attachment is represented as text

- **GIVEN** a turn whose prompt contains an `attach:` link
- **AND** the turn's attachments map holds a non-image payload for that link's id
- **WHEN** the harness assembles the prompt for the turn
- **THEN** the attachment is represented as a text content block
- **AND** the content does not include an image content block for that attachment

> test: code

### Scenario: An unresolved attach marker is left literal

- **GIVEN** a turn whose prompt contains an `attach:` link
- **AND** the turn's attachments map has no entry for that link's id
- **WHEN** the harness assembles the prompt for the turn
- **THEN** the original markdown link remains as text content

> test: code
