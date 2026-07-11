# Uniform ACP harness

Duckboard drives every coding harness through one ACP client; Claude Code is reached via
an owned ACP agent child over the official CLI, while Grok stays a direct native ACP spawn
— with no npm or other foreign runtime.

## Motivation

Claude and Grok already share duckchat’s provider and warm-runtime traits, but they do not
share a wire protocol: Claude is still a cold `stream-json` CLI client, while Grok is
process-hot ACP. That split doubles maintenance (two event paths, two session models, two
heat stories) and leaves Claude without the warm-process benefits Grok already has.

Why now: the Grok ACP client and warm runtimes are in place and proven; exploration ruled
out npm-based Claude ACP adapters and a pass-through Grok proxy. The remaining path is
clear — own the client, own a Claude ACP agent child, leave native Grok as the agent
process — and doing it before more harness work lands avoids cementing the dual stack.

## Intent

- One ACP client runtime runs all harness turns: initialize, session open/resume, prompt,
  streaming updates, cancel, and process heat

- Selecting Claude or Grok only changes which agent process is spawned, not which client
  stack runs the turn

- Claude Code remains the real Claude backend (official `claude` CLI behind our agent);
  auth, tools, and skills stay with Claude Code

- Our Claude path is a child ACP agent process that translates to/from that CLI — no
  Node/npm and no reimplementation of Claude as a product

- Grok continues to be spawned as its own first-party ACP agent (no intermediate proxy we
  own “just for uniformity”)

- Existing harness identity and session binding stay meaningful: a session remains tied to
  one harness; resume uses that harness’s session ids

- When the change is done, the Claude stream-json client path in the host is gone; both
  harnesses are warm ACP clients from duckboard’s point of view

## Non-goals

- A Grok (or generic) ACP proxy whose only job is to forward to the native agent

- Rebuilding Claude Code (custom tool loop over the Messages API as a substitute for the
  CLI)

- Depending on npm, Node, or other cross-language runtimes for the Claude path

- First-party Anthropic ACP support (use it later if it appears; do not block on it)

- Interactive permission UI or structured agent→user question bridging in duckboard

- Adding further harnesses beyond making Claude and Grok share the ACP client pattern

- Changing duckspec slash-command install targets or the `ds init` harness list
