# README harness requirements and install links

User-led followup after full implementation: top-level README should document requirements
and official install links for every supported harness.

## Scope

Post-implementation followup on `openai-codex-harness`. Discussed `README.md` Installation
/ Quick start / Development sections against the three duckboard backends (Claude, Grok,
OpenAI Codex) and `ds init` harnesses (`claude`, `opencode`, `codex`).

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | quality | README lacks per-harness requirements and install links | /ds-step |
```

## Issues

### 1. README lacks per-harness requirements and install links - quality/major

**Where:** `README.md` Installation, Quick start, and Development (agent binary discovery
table)

**Why:** After shipping Codex, users still cannot see in one place which external CLIs
each harness needs, how to install them, or where official docs live. The README covers
`ds` / duckboard / owned ACP agents (`duckchat-*-acp`) but not the upstream backends:
Claude Code (`claude`), Grok Build (`grok`), Codex CLI (`codex`). That blocks onboarding
for Codex and leaves Claude/Grok similarly under-documented. `ds init` also supports
`opencode` without a parallel requirements note.

**Action:** Extend the README with a scannable per-harness section (table or short
subsections) covering at least:

```
| Harness (duckboard / init) | Requires (on PATH) | Official install / docs |
| --- | --- | --- |
| Claude (`claude-code` / `ds init claude`) | `claude` | [Claude Code quickstart](https://code.claude.com/docs/en/quickstart) |
| Grok (`grok`) | `grok` | [Grok Build CLI](https://x.ai/cli) / [docs](https://docs.x.ai/build/overview) |
| OpenAI Codex (`openai-codex` / `ds init codex`) | `codex` | [Codex CLI](https://developers.openai.com/codex/cli) |
| OpenCode (`ds init opencode` only) | OpenCode agent as used for skills | project’s OpenCode install docs if we list it |
```

Also: auth is the upstream product’s (ChatGPT / Claude / SuperGrok etc.); duckboard does
not replace that. Keep owned-agent discovery (`DUCKCHAT_*_ACP`, sibling binaries) distinct
from upstream CLI install. Pure doc rework — plan via `/ds-step` (or edit in place if
preferred after confirm).

## Outcome

Agreed: README should state requirements and installation links for each supported
harness. Plan and code unchanged in this session. Suggested next: `/ds-step` to plan the
README edit (or apply the doc change directly if you want to skip a formal step). Not
archive-ready until this lands or is explicitly deferred.
