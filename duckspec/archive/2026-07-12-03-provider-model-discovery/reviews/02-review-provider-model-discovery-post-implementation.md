# Provider model discovery post-implementation

Post-implementation review of live catalog discovery, process catalog, and oneshot
settings after steps 01–07. Audit clean (20/20). One residual major oneshot preference
mismatch; otherwise archive-ready with small follow-ups.

## Scope

`proposal.md`, `design.md`, caps deltas (claude/grok/model-catalog/oneshot-models), steps
01–07, followup 01, and code in `duckchat-claude-acp` models/agent, `duckchat`
claude/grok/worker/provider, `duckboard` agent/settings/config/main.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | soundness | Oneshot preferred alias vs live full model ids | /ds-step |
| 2 | minor | quality | Catalog refresh never wakes the UI | /ds-step |
| 3 | minor | quality | Provider OnceLock freezes first empty discovery | ignore |
```

## Findings

### 1. Oneshot preferred alias vs live full model ids - soundness/major

**Where:** `crates/duckchat/src/claude_code.rs` and `grok.rs` `open_oneshot_runtime`
(`preferred_model.or_else(|| Some(TITLE_MODEL))`); `crates/duckchat/src/acp/runtime.rs`
`pick_oneshot_model` (exact id match); host `agent::resolve_oneshot_model` uses substring
match on full catalog ids.

**Why:** Live Claude advertise uses API model ids (e.g. `claude-haiku-4-5-…`). Preferred
`"haiku"` fails exact match, so oneshots fall through to the first advertised model (often
Sonnet). That path is hit when the agent subscription is built with an empty catalog
(`None` → bare alias), and when transitional `title_summary` / `reply_suggestions` still
hardcode `TITLE_MODEL`. Settings can show the correct full id (after catalog fill + step
07) while an already-spawned worker still prefers the bare alias. Lasting cost: expensive
or wrong oneshot model in the common live-catalog world.

**Action:** Prefer only catalog-resolved full ids into the oneshot path (no bare-alias
fallback when advertise uses full ids), or align `pick_oneshot_model` with the same
string-match needles as host resolve; rebuild the worker preferred model when the catalog
becomes non-empty. Plan via `/ds-step`.

### 2. Catalog refresh never wakes the UI - quality/minor

**Where:** `crates/duckboard/src/agent.rs` `start_model_catalog_refresh` (background
thread with no iced message).

**Why:** First paint can keep empty model/oneshot pickers until some other interaction
re-renders. Design allowed a brief empty window; still a durable UX cliff when discovery
is slow or the user opens Settings immediately.

**Action:** Emit a lightweight “catalog ready” message (or block first model UI on refresh
completion) if empty-first becomes common. Optional `/ds-step`.

### 3. Provider OnceLock freezes first empty discovery - quality/minor

**Where:** `ClaudeCodeProvider` / `GrokProvider` model memo (`OnceLock`).

**Why:** A failed first `list_models` sticks for the process lifetime; design wording
about refresh rediscovering is not implemented. Low risk with app-start-only refresh, but
recovery requires restart.

**Action:** Accept for now, or replace with a refreshable memo if empty-first becomes
common. Suitable to `ignore` unless observed in practice.

## Verdict

**Accept with residual work.** Intent is realized: Claude host static list removed, live
advertise with curated fallback, process catalog, global oneshot config, and settings
pickers aligned with the resolve ladder (followup 01 addressed in step 07). Residual major
is oneshot preferred-id consistency under live full ids and catalog race. Prefer
`/ds-step` for finding 1 before archive; findings 2–3 optional.
