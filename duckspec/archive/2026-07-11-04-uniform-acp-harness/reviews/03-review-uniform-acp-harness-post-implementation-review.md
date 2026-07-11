# Uniform ACP harness — post-implementation review

Post-implementation review of `uniform-acp-harness` after shared ACP client, Claude agent,
host cutover, defer-spawn hang fix, progressive stream, and thinking map. Architecture
matches design; audit green. Main gap: released GUI packaging and install docs still omit
a clear agent binary story.

## Scope

Proposal, design, change caps (`harness/acp-client`, `harness/claude`, grok deltas), steps
01–08, followups 01–02, code under `crates/duckchat/`, `crates/duckchat-claude-acp/`,
`justfile` bundle/install/release, `.github/workflows/release.yml`, and README install
sections. Prior followups as context.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | soundness | Agent binary not packaged with Duckboard; install docs incomplete | /ds-step |
| 2 | minor | quality | Agent mixes tokio and std stdout writers for ACP | /ds-step |
| 3 | minor | quality | Progressive delivery untested across ACP stdio | ignore |
```

## Findings

### 1. Agent binary not packaged with Duckboard; install docs incomplete - soundness/major

**Where:** `justfile` `bundle` copies only `duckboard` into `Contents/MacOS/`; release
workflow builds `-p duckboard` only; `just install` installs three binaries but README
primary install paths (DMG / `cargo install duckboard` / ds tarball) do not tell users to
install `duckchat-claude-acp` or place it beside the GUI.

**Why:** Discovery is env → sibling of exe → PATH. A DMG app without a sibling agent, and
a user who only installed `ds` + `duckboard` without the agent, get missing-binary Claude
turns. Design assumed co-shipping for the GUI path.

**Action:** Prefer packaging: build and copy `duckchat-claude-acp` next to `duckboard` in
the app bundle (and release CI). Also update README installation for non-bundle paths
(e.g. `cargo install` / `just install` listing the agent, or an explicit “Claude turns
need `duckchat-claude-acp` on PATH or beside duckboard”). Runtime auto-download of the
agent is out of scope. `/ds-step`.

### 2. Agent mixes tokio and std stdout writers for ACP - quality/minor

**Where:** `crates/duckchat-claude-acp/src/main.rs` — tokio stdout for results;
`std::io::stdout` for mid-turn `session/update`.

**Why:** Two writers on one stdio stream are harder to reason about and can reorder/buffer
oddly under load.

**Action:** Single stdout path for all ACP lines. Polish `/ds-step` or accept as debt.

### 3. Progressive delivery untested across ACP stdio - quality/minor

**Where:** Progressive `@spec` is in-process agent unit test; `tests/client_turn.rs` does
not assert update-before-result on the wire.

**Why:** Batching regression in `main.rs` sync write could slip.

**Action:** Optional `client_turn` assertion, or leave as `ignore` if GUI check is enough.

## Open questions

None that block the review. (Runtime auto-install of the agent from the app is explicitly
out of scope.)

## Verdict

**Accept architecture; not fully archive-ready for the shipped GUI product path.** Shared
ACP + Claude agent + host cleanup match the proposal; hang fix and progressive/thinking
followups land. Ship agent beside duckboard **and** document non-bundle install (finding
1) before treating released Duckboard Claude as done; 2–3 are small quality debt.
