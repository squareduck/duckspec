# Review: Codex full local access

The change is ready to archive. Its design, capability contract, implementation, and
focused verification consistently establish unconditional full local access for every
Codex turn.

## Scope

Reviewed the proposal, design, OpenAI Codex capability deltas, completed implementation
step, source changes in the owned ACP agent and app-server adapter, focused tests, Clippy
results, working-copy diff, and duckspec mechanical checks.

## Summary

No accepted findings remain.

## Resolved concerns

The unrestricted policy is explicitly attached to every `turn/start` request as
`{"type":"dangerFullAccess"}`. Consecutive hot-process turns and a resumed thread after
restart both exercise the actual serialized payload.

Policy rejection surfaces through the existing app-server error path. The scripted backend
rejects only the first request, so the passing rejection test also establishes that the
agent does not retry with a fallback policy.

Removing repository-access discovery and session state does not remove working-directory
behavior: new threads still receive the ACP working directory, while resumed threads
retain their persisted Codex context.

The capability documentation accurately describes the machine-level security boundary,
including access to unrelated user resources and the fact that project instructions do not
provide operating-system isolation.

`ds check` and `ds audit codex-full-local-access` pass. All 35 focused
`duckchat-codex-acp` tests pass, and focused Clippy passes with warnings denied. Workspace
testing reaches the previously recorded environment limitation: Cargo cannot create the
missing `clap` package directory in the sandboxed user registry.

## Outcome

Ready to archive. No design, specification, or implementation correction is required.
