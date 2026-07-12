# Post-implementation review of openai-codex harness

Reviewed `openai-codex-harness` end-to-end after all eight steps (including the README
followup). Architecture matches design; unit tests pass. Freeze is blocked by catalog
unavailability lying about a missing backend and a brittle auto-approval response map.

## Scope

Full chain: `proposal.md`, `design.md`, `caps/harness/openai-codex`,
`caps/cli/stock-content` deltas, steps 01–08, prior followup
`reviews/01-followup-readme-harness-requirements-and-install-links.md`, and implementation
in `crates/duckchat-codex-acp/`, `crates/duckchat/src/openai_codex*`, duckboard
registration/packaging, `ds init codex`, and root `README.md`.

Post-implementation; deepest layer is code. `ds audit openai-codex-harness` and `ds check`
are clean. Spec-linked unit tests for the agent, provider, and stock content pass.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | fidelity | Missing Codex still advertises curated models | /ds-step |
| 2 | major | soundness | Permissions approval auto-reply uses wrong shape | /ds-step |
| 3 | minor | quality | Multi-question user-input only answers the first | /ds-step |
| 4 | minor | quality | Stale agent shell docs and unused turn_interrupt | ignore |
```

## Findings

### 1. Missing Codex still advertises curated models - fidelity/major

**Where:** `crates/duckchat-codex-acp/src/models.rs` (`resolve_advertised_models` /
`curated_fallback`); `Agent::initialize` in `crates/duckchat-codex-acp/src/agent.rs`;
requirement Graceful unavailability in `caps/harness/openai-codex/spec.md` and matching
doc prose

**Why:** Spec and doc require empty model discovery when the official Codex backend (or
auth) is unavailable. Live `model/list` failure always falls through to a non-empty
curated list (`gpt-5.6-*`, `gpt-5.4`, `gpt-5.4-mini`, …). With the owned agent shipped
next to duckboard, a machine that has `duckchat-codex-acp` but no `codex` on PATH still
populates the openai-codex picker slice. Users pick models that cannot start a turn — the
opposite of “degrades safely with no models.” Claude’s curated aliases still run through a
present `claude`; Codex curated ids do nothing without app-server. The provider test only
covers a missing *agent* binary, so this hole is green in CI.

**Action:** Advertise curated fallback only when app-server is reachable (or when
`model/list` succeeds empty in a recoverable way). If spawn/`model/list` fails because
`codex` is missing or unusable, return an empty advertise set so host `list_models` is
empty; keep typed turn errors. Add a scripted-agent or factory test for “agent up, backend
down → no models.”

### 2. Permissions approval auto-reply uses wrong shape - soundness/major

**Where:** `crates/duckchat-codex-acp/src/codex/ask_user.rs`
(`is_ordinary_approval_method`, `auto_allow_approval_result`); demux in
`crates/duckchat-codex-acp/src/codex/app_server.rs` `handle_incoming`

**Why:** Any non-user-input method whose name contains `Approval`/`approval` is auto-
answered with `{ "decision": "accept" }` (or legacy `"approved"` for exec/patch names).
Official App Server also issues `item/permissions/requestApproval` for network/filesystem
grants from the built-in permissions tool; clients are expected to return a **permission
grant** body (promptfoo’s non-interactive default is an empty grant), not a
command-execution `decision`. A wrong result shape can stall or fail the turn when
elevated grants are requested even under `approvalPolicy: "never"`. Freezing this broad
auto-allow as “ordinary tools stay auto-approved” papers over a distinct server-request
class.

**Action:** Branch methods: keep `decision` accept/approved for command/file/exec
approvals; handle `item/permissions/requestApproval` (and any documented grant shape) with
an explicit auto-grant policy that matches App Server, or decline safely with the
documented empty/denied grant. Cover both shapes in unit tests.

### 3. Multi-question user-input only answers the first - quality/minor

**Where:** `crates/duckchat-codex-acp/src/codex/ask_user.rs` `decode_user_input` /
`answers_map`; parent bridge in `crates/duckchat-codex-acp/src/agent.rs`
`service_parent_choice`

**Why:** App Server `tool/requestUserInput` (and `item/…` variants) may carry 1–3
questions. The agent surfaces only the first question’s options and writes a single entry
into the answers map. Spec scenarios are single-question, and design treats the API as
experimental, but freezing first-only means multi-question tools complete with partial
answers and can error or hang in real sessions.

**Action:** Either encode all questions (loop host choices or one composite UI) or
document first-only as an explicit adapter limit in the harness doc and leave a followup.
Prefer full answers if cheap; otherwise name the limit so it is not silent.

### 4. Stale agent shell docs and unused turn_interrupt - quality/minor

**Where:** `crates/duckchat-codex-acp/src/main.rs` crate docs (“Backend wiring lands in
later steps”); `AppServer::turn_interrupt` marked `dead_code` while design cancel sketch
is interrupt then kill heat

**Why:** Misleading module docs and a dead interrupt helper cost little today but teach
the next editor the wrong lifecycle. Host cancel already kills the ACP child (Claude
pattern), so behavior is acceptable; craft debt only.

**Action:** Refresh main docs to describe the live App Server bridge; either call
`turn/interrupt` before kill when a turn id is tracked, or drop the dead helper and
document kill-only cancel in the harness doc.

## Open questions

None that block the findings above. Whether multi-question must be full parity in v1 is a
product call once finding 3’s limit is explicit.

## Verdict

Not ready to archive. The owned ACP → app-server shape, thin provider, duckboard
registration, packaging, `ds init codex` skills, and README harness table implement the
design and close the prior README followup. Fix empty-catalog-on-missing-backend and
permission-grant auto-replies before freeze; multi-question and doc nits are optional
polish.
