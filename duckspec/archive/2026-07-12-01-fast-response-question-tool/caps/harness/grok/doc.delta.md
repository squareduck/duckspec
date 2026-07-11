# @ Grok harness

## ~ Turn lifecycle

The main path uses the shared ACP client with a Grok launch
(`grok agent --always-approve
stdio`, without `--no-ask-user`). The client keeps that
agent process warm across turns when possible, opens or resumes a grok session, and
prompts:

```
[if cold] spawn grok launch + initialize
   │
session/new | session/load     (shared client)
   │
session/prompt                 stream profile updates → agent events
   │                           structured questions → host user choice
   │
process stays up               (until cancel or handle shutdown)
```

The session id grok assigns is harness-bound: it cannot be resumed by another backend.
Cancel kills the main agent child; the next turn may spawn again and still resume that id
when supplied.

Tool execution is auto-approved for the turn. Structured questions use the xAI extension
`x.ai/ask_user_question` and surface through the shared client's main-path user-choice
loop (accepted answers for chip or custom freeform, skip-interview on cancel). Custom
freeform completes as accepted with free text as the answer value, not skip-interview.
Ordinary allow/reject tool permission prompts stay auto-allowed by the client.
