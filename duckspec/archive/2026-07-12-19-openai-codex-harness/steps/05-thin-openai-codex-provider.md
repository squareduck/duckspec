# Thin openai-codex provider

Add `OpenaiCodexProvider` in duckchat: launch owned agent, model discovery, oneshot
preference, `.agents/skills` command discovery, graceful unavailability.

## Prerequisites

- [x] @step attachments-questions-and-auto-approve-tools

## Tasks

- [x] 1. Add `crates/duckchat/src/openai_codex.rs` (+ agent_bin / skills discover) with
         harness id `openai-codex`, launch, and oneshot preferred model `gpt-5.4-mini`

- [x] 2. Wire models from agent initialize; empty list on discovery failure; list_commands
         from `.agents/skills/*/SKILL.md`

- [x] 3. @spec harness/openai-codex Owned ACP agent over official Codex: A Codex turn is driven through the owned ACP agent process

- [x] 4. @spec harness/openai-codex Model discovery and oneshot preference: Discovered models are tagged with the openai-codex harness

- [x] 5. @spec harness/openai-codex Model discovery and oneshot preference: Each listed model carries a display name

- [x] 6. @spec harness/openai-codex Model discovery and oneshot preference: Preferred oneshot model is selected when advertised

- [x] 7. @spec harness/openai-codex Model discovery and oneshot preference: Oneshot model falls back when preferred is absent

- [x] 8. @spec harness/openai-codex Graceful unavailability: A missing agent or backend yields no models and a turn error

- [x] 9. @spec harness/openai-codex Stage skill discovery: Skills under .agents/skills are listed as slash commands

- [x] 10. @spec harness/openai-codex Stage skill discovery: A project without .agents/skills yields an empty command list
