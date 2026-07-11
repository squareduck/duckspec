# @ Claude harness

## @ Process tree

```
duckboard / duckchat worker
   │  ACP (shared client) — mid-turn user choice when Claude asks
   ▼
duckchat-claude-acp          (owned agent)
   │  stream-json duplex + control / canUseTool
   ▼
claude                       (official CLI)
```

Selecting the Claude harness only changes the provider launch (the agent binary). Turn
lifecycle, event mapping, and main heat for the **agent** process are the shared ACP
client. This capability owns Claude-specific behavior: agent binary discovery, when the
inner `claude` process starts, Claude-native session ids after the first prompt, duplex
heat of that process, translating Claude's stream into the client's dialect profile, and
bridging AskUserQuestion to the parent's user-choice loop.

## + Structured questions

Claude may call `AskUserQuestion` during a turn. The owned agent is not configured to
disallow that tool. When Claude asks, the agent maps the control / canUseTool request to a
parent ACP choice so the host can show options. A host selection completes as allow with
`updatedInput` carrying the original `questions` and an `answers` map (question text →
selected option label). A host custom freeform answer completes the same way with free
text as the answer value (not deny). Host cancel finishes without accepting the
questionnaire.

Ordinary tools stay on permission bypass: they do not open the host choice UI. Only
structured clarifying questions use the mid-prompt parent choice path.
