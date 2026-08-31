# Codex full local access - Design

Make unrestricted local access an explicit, unconditional per-turn contract at the Codex
app-server boundary, while removing repository-specific sandbox state from the owned ACP
agent.

## Turn access policy

`crates/duckchat-codex-acp/src/codex/app_server.rs` owns the policy sent to the official
Codex app-server.

```
ACP session/prompt
        │
        ▼
Agent::run_turn_stream
        │
        ▼
AppServer::turn_start
        │
        ├── sandboxPolicy.type = dangerFullAccess
        └── model + input
```

Every `turn/start` request supplies:

```json
{
  "sandboxPolicy": {
    "type": "dangerFullAccess"
  }
}
```

The policy is hardcoded rather than inherited from the user's Codex configuration or
exposed as a Duckboard setting. This gives all Duckboard Codex turns one predictable
access contract.

`approvalPolicy: "never"` remains on thread start and resume. Automatic handling of
ordinary app-server approval requests also remains as protocol compatibility for command,
file, and permissions requests. Structured user-input requests continue through the
existing host-choice path.

## App-server interface

The turn API no longer accepts repository-derived writable roots:

```rust
pub async fn turn_start(
    &mut self,
    thread_id: &str,
    input: Vec<Value>,
    model: Option<&str>,
) -> Result<String, AppServerError>;
```

Sandbox policy construction stays private to `AppServer::turn_start`, keeping the security
boundary at the protocol adapter instead of distributing it through agent orchestration.

## Agent state

The owned ACP agent removes repository access as session state:

```
Removed
├── RepositoryAccess
├── Agent.repository_access
├── .git and .jj discovery
├── canonicalized repository roots
├── session-open/load access refresh
└── per-turn repository-context lookup
```

`session_new` still passes the ACP working directory to `thread/start`. Session identity,
thread resumption, process heat, cancellation, model selection, prompt assembly, and
streaming remain unchanged.

A resumed Codex thread continues to use its persisted working-directory context.
App-server restarts require thread resumption as before, but no repository access state
needs rebuilding.

## Failure behavior

A rejected `dangerFullAccess` policy follows the existing app-server error path. The agent
does not retry with a workspace sandbox, inherited configuration, missing policy, or any
other fallback.

Removing the repository-context lookup also removes its local "no repository context"
failure. Once a session is valid, access-policy acceptance is decided only by the
app-server.

## Verification

Protocol-focused tests replace repository-metadata tests:

- Every turn sends exactly the `dangerFullAccess` policy.

- Consecutive turns each send the policy even when reusing a process-hot app-server.

- A resumed thread sends the policy after an app-server restart.

- Policy rejection surfaces without retry or fallback.

- Existing approval, structured-input, session, cancellation, and streaming coverage
  remains unchanged.

Scripted app-server fixtures continue exposing received `sandboxPolicy` values through
test updates, so assertions verify the actual protocol payload rather than an internal
helper.

## Documentation boundary

The OpenAI Codex capability replaces repository-scoped VCS access with full local access.
Its specification and documentation state that Codex receives the permissions of
Duckboard's OS user, including access beyond the selected repository.

Project instructions constrain intended behavior but do not provide isolation. This change
adds no settings control or warning dialog: access is unconditional, and the current UI
does not claim repository containment.
