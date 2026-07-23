# Codex VCS access and accurate usage

The change is consistent across design, capability contracts, implementation, and tests.
No corrective findings remain.

## Scope

Reviewed the proposal, design, capability deltas, both completed implementation steps,
Codex agent source, request construction, telemetry mapping, and linked tests.

Verification included:

- `ds check` - passed

- `ds audit codex-vcs-access-and-usage` - passed

- `cargo test -p duckchat-codex-acp` - 36 passed

- Installed Codex app-server schema - confirmed the workspace-write `sandboxPolicy` and
  `writableRoots` request shape

Ordinary `jj status` could not snapshot the working copy because the current sandbox
denied writes to `.git/objects`. `jj status --ignore-working-copy` reported the last
recorded snapshot as clean, so the unsnapshotted working-copy diff remains an
environmental review limitation.

## Summary

```
| # | finding | resolution | → next |
| --- | --- | --- | --- |
| — | No accepted findings | Change is ready to archive | `/ds-archive` |
```

## Outcome

Ready to archive. Repository metadata discovery, lifecycle retention, per-turn sandbox
policy, failure behavior, and latest-turn usage preference conform to the approved design
and capability contracts.
