# OpenAI Codex harness

Add OpenAI Codex as a third duckboard agent backend with the same chat-turn parity as
Claude and Grok, by owning an ACP agent over the official Codex app-server rather than
depending on a community runtime adapter — and let projects install duckspec agent stages
for Codex via `ds init codex` into Codex’s native skill layout.

## Motivation

Duckboard already drives two coding agents behind one harness abstraction (shared ACP
client, model catalog, warm main/oneshot paths, attachments, mid-turn questions, usage
meter). Codex is a third frontier agent many users already run, but it is not selectable
in duckboard, so those chats stay outside the product.

Codex also does not natively load `.claude` skills or commands; it discovers repo skills
under `.agents/skills`. Without a Codex init path, duckspec stages that Claude/OpenCode
get from `ds init` never land where Codex looks.

Why now: the shared ACP client and Claude’s owned-agent pattern are proven. Codex’s real
integration surface is App Server (not native ACP), so the remaining work is an owned
bridge into that surface — better done before more harness-specific UI piles onto only
Claude and Grok. A small `ds init codex` that targets `.agents/skills` is the natural
install seam for the same stage content.

## Intent

- Duckboard can run full chat turns on OpenAI Codex with the same host experience as
  Claude and Grok: model pick, stream (text, tools, reasoning when present), cancel,
  session resume, warm main and oneshot paths, image attachments, mid-turn structured
  questions answered in-band, usage/context fill, and graceful absence when Codex is
  missing or unauthenticated

- The host keeps one ACP client stack; Codex is not a second in-host wire protocol. The
  product owns the ACP agent process that talks to official Codex (no npm/Node community
  adapter at runtime)

- Codex models are a first-class harness group in the picker; identity and session binding
  stay harness-scoped so resume cannot cross backends

- Missing or unauthenticated Codex degrades safely (no models / typed turn errors),
  without breaking Claude or Grok

- `ds init codex` installs duckspec agent stages into Codex’s native skill layout under
  `.agents/skills`, so Codex can discover and use them the same way other harnesses get
  stock stage content from init

## Non-goals

- Community or npm-based Codex ACP adapters as the shipped path

- Teaching the host a second primary wire protocol instead of an owned ACP agent

- Live dual-read of `.claude` from the Codex process (Codex keeps its own skill roots;
  init writes where Codex looks)

- MCP, sandbox, or permission-mode configuration UI beyond what parity requires for
  auto-approved ordinary tools

- Codex-only product surfaces (review mode, goals, plugins, remote environments, realtime)
  as first-class duckboard chrome

- Changing the global default model cascade away from its current behavior

- Renaming or reworking the knowledge-tree “codex” area (harness naming must not collide
  with it)

- Broader multi-agent config sync (MCP, permissions, CLAUDE.md ↔ AGENTS.md migration
  tooling)
