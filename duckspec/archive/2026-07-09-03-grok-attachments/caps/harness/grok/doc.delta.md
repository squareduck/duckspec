# @ Grok harness

## + Prompt attachments

A turn request may carry binary attachments keyed by id, with the prompt text referring to
them as markdown links of the form `[label](attach:<id>)`. Before sending
`session/prompt`, the harness walks those markers and builds a multi-block ACP content
array instead of a single unexpanded text string.

```
prompt text + attachments map
        │
        ▼
  walk [label](attach:<id>)
        │
        ├── resolved image/*  →  ACP image block (mimeType + base64 data)
        ├── resolved other    →  text block naming the attachment
        └── unresolved id     →  original markdown left as text
        │
        ▼
  session/prompt.prompt: [ text | image | text | … ]
```

System-prompt additions still fold into the leading text (blank-line separated) ahead of
the user message; attach markers normally appear only in that user message text.
Selection-context chips are separate: they are plain text prepended by the caller and do
not use the attachments map.
