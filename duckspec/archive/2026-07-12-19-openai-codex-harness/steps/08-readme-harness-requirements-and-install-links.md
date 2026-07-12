# README harness requirements and install links

Add a scannable per-harness section to the root README: upstream CLI requirements,
official install/docs links, and a short note that auth is upstream (owned ACP agents stay
separate).

## Prerequisites

- [x] @step ds-init-codex-stock-skills

## Context

From followup `reviews/01-followup-readme-harness-requirements-and-install-links.md`.
Doc-only; no code or cap changes.

Suggested official links (verify still current when writing):

- Claude Code: https://code.claude.com/docs/en/quickstart
- Grok Build: https://x.ai/cli and https://docs.x.ai/build/overview
- Codex CLI: https://developers.openai.com/codex/cli

## Tasks

- [x] 1. Add a scannable per-harness section near Installation or Quick start covering
         Claude, Grok, OpenAI Codex, and `ds init opencode` as appropriate

- [x] 2. Present harness id / `ds init` name → binary on PATH → official install/docs link
         (table or tight subsections)

- [x] 3. Note that auth is the upstream product’s; keep `DUCKCHAT_*_ACP` / sibling agent
         discovery distinct from upstream CLI install

- [x] 4. Align Quick start and any `ds init` help text that still omits harness
         prerequisites
