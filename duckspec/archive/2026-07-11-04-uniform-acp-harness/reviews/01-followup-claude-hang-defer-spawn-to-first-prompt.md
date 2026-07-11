# Claude hang: defer spawn to first prompt

User-led followup after implementation: Claude chat hangs forever on send; Grok still
works. Root cause is agent open waiting for a session id before any user content; agreed
fix is option 1 (spawn Claude on first prompt, rebind native session id).

## Scope

Post-implementation on `uniform-acp-harness` (all four steps done, audit clean). Discussed
host ACP open/prompt split, `duckchat-claude-acp` duplex open (`wait_for_session_id`
before write), and live CLI probes (no init without user message; init+turn after first
write). Agreed architectural choice: defer inner Claude spawn to first `session/prompt`.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | critical | soundness | Claude hang: open waits for init before first user message | /ds-step |
```

## Issues

### 1. Claude hang: open waits for init before first user message - soundness/critical

**Where:** `crates/duckchat-claude-acp/src/claude/duplex.rs` (`open_new` /
`spawn_and_init` → `wait_for_session_id`); `agent.rs` `session_new` blocks on that before
returning; host `AcpMainRuntime::run_turn` never leaves `open()` so no `SessionIdUpdated`
/ tokens. Live `claude -p` duplex: zero stdout until a user stream-json line is written.

**Why:** Every Claude main turn hangs indefinitely; silent (Claude stderr nulled; no open
timeout when zero lines). Blocks shipping and archive.

**Action:** Prefer **option 1** — defer official `claude` spawn until first
`session/prompt`. `session/new` returns a provisional ACP handle without starting Claude;
first prompt spawns duplex, writes user content, reads init+stream, binds Claude’s native
session id; host surfaces that id on turn outcome / session update for resume. Keep duplex
heat after first bind. Plan via `/ds-step` (and `/ds-spec` only if cap/session wording
must change); implement with `/ds-apply`. Do not ship cold-inner unless multi-turn reuse
fails after this fix.

## Outcome

Agreed: hang is soundness/critical; fix path is defer-spawn (1), not
spawn-on-open-without-wait (2). Plan/code unchanged in this followup write. Not
archive-ready until the fix lands.
