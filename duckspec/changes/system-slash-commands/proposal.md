# System slash commands

Duckboard owns local system slash commands (starting with `/help`), tags every slash entry
by kind, paints system vs workflow vs agent differently, and teaches a `//` escape so
harness help stays reachable.

## Motivation

`/help` appears in slash completion (via shared Claude builtins discovery) but is always
sent as a normal agent turn. With the Grok harness that becomes Grok’s help skill — Grok
docs, not duckboard/duckspec help. Claude-only builtins (`clear`, `compact`, `cost`,
`help`, `model`) are also advertised on Grok and mostly do not do what their descriptions
claim.

Users need a clear split: what duckboard runs locally, what is duckspec workflow
(`/ds-*`), and what is a harness skill — both in the completion UI and in the transcript
when a system command runs.

## Intent

- Bare `/help` is handled entirely by duckboard: no agent turn, no priming path, no burn
  of tentative selection context

- A short system notice states that a system command ran and how to get agent/harness help

- Slash completion entries are kinded (system / workflow / agent) and colored so kinds are
  scannable before send

- System commands are owned by duckboard’s registry, not by provider discovery; fake
  Claude builtins are not advertised on harnesses that cannot honor them

- Name collisions: bare `/name` prefers the system handler; agent copy of that name is
  reachable only via escape

- Double-slash escape: `//help` forces `/help` to the agent while the transcript keeps the
  typed form when possible

- Help body lists the live completion catalog by kind (system, workflow, agent) with the
  active harness labeled on the agent section

## Non-goals

- Full keybind browser or settings dump as help
- Implementing every former Claude builtin in v1 (`/clear`, `/model`, `/cost`, `/compact`)
- Changing Grok ACP or Claude `-p` protocol
- Rewriting agent skill markdown files
- Composer token tinting while typing (optional later polish)
