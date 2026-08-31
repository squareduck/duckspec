# Grant Codex full local access

Replace the repository workspace sandbox with an explicit unrestricted per-turn policy and
remove obsolete repository-access state.

## Tasks

- [x] 1. Change `AppServer::turn_start` to send `dangerFullAccess` without writable roots
         and simplify its signature.

- [x] 2. Remove `RepositoryAccess`, repository metadata discovery, stored repository
         context, and associated session and turn plumbing from `Agent`.

- [x] 3. Update the scripted app-server fixture and policy-capture helpers for the
         unrestricted policy while preserving approval and question behavior.

- [x] 4. @spec harness/openai-codex Full local access: Every turn receives full local access

- [x] 5. @spec harness/openai-codex Full local access: A resumed session reapplies full local access after restart

- [x] 6. @spec harness/openai-codex Full local access: A rejected full-access policy does not trigger a fallback

- [x] 7. Run focused crate tests and workspace validation, fixing regressions without
         changing the confirmed access contract.

## Outcomes

Focused tests and Clippy pass for `duckchat-codex-acp`. The workspace check could not
unpack a missing Cargo dependency into the sandboxed user registry; it reported no source
error before that environment failure.
