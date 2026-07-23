# Codex VCS access and accurate usage - Design

Apply an explicit repository-scoped sandbox policy to every Codex turn and normalize Codex
telemetry from latest-turn usage rather than cumulative thread consumption. The Codex ACP
agent owns both adaptations; shared ACP and Duckboard behavior remain harness-neutral.

## Repository access policy

The normalized ACP working directory is the repository boundary. The Codex agent discovers
only existing `.git/` and `.jj/` directories directly beneath that root.

```
repository root
│
├── ordinary contents   workspace-write already permits these
├── .git/                explicit writable root when it is a directory
└── .jj/                 explicit writable root when it is a directory
```

A small repository-access value carries the normalized root and ordered writable metadata
roots:

```rust
struct RepositoryAccess {
    root: PathBuf,
    writable_roots: Vec<PathBuf>,
}
```

Discovery is filesystem-only. It does not invoke `git` or `jj`, search ancestors, follow a
`.git` file, or grant access to metadata outside the repository. Missing metadata
directories produce an empty additional-root list.

This intentionally excludes Git worktrees whose `.git` indirection points to an external
store. Supporting external stores would require a separate opt-in because it crosses the
repository boundary.

## Session state and lifecycle

The Codex agent separates app-server process membership from repository access context:

```
Codex thread id
├── hot-process membership      cleared when app-server heat ends
└── repository access context   retained for the ACP agent lifetime
```

`session/new` and `session/load` both receive the normalized `cwd` from the shared ACP
client. Each operation discovers the current metadata directories and stores the resulting
access context by Codex thread id.

Refreshing on `session/load` means metadata created after the session opened becomes
available on the next turn. It also reconstructs access after the ACP agent itself
restarts.

When the app-server process restarts, only hot-process membership is cleared. The thread
is resumed as today, and the next turn receives the retained repository policy.

## Turn policy

`AppServer::turn_start` accepts the repository access context and includes an explicit
sandbox policy in every request:

```json
{
  "threadId": "<thread>",
  "input": ["<mapped prompt blocks>"],
  "sandboxPolicy": {
    "type": "workspaceWrite",
    "writableRoots": [
      "<repository>/.git",
      "<repository>/.jj"
    ]
  }
}
```

The roots array contains only directories discovered for that repository and may be empty.
Existing model selection and input mapping remain unchanged.

`approvalPolicy: "never"` remains the ordinary-tool approval behavior established on the
thread. The sandbox policy supplies filesystem capability; duckspec workflow and project
instructions remain responsible for deciding when the agent may commit or perform
destructive VCS operations.

The policy is sent on every turn rather than relying on sticky app-server state. New
sessions, resumed sessions, process-hot turns, and turns after backend restart therefore
share the same access contract.

If the backend rejects the policy, the existing typed app-server error path surfaces the
failure. The harness does not retry with broader access or silently omit the policy.

## Usage normalization

Codex token notifications contain two different totals:

```
tokenUsage.last.totalTokens    latest turn
tokenUsage.total.totalTokens   cumulative thread consumption
```

The Codex notification mapper changes its preference order:

```
last.totalTokens
      │ present
      ▼
_meta.totalTokens
      ▲
      │ last absent
total.totalTokens
```

The cumulative value is only a compatibility fallback for older or incomplete payloads. If
neither value exists, no usage update is emitted.

The adapter continues emitting the shared `_meta.totalTokens` profile shape. The shared
ACP mapper continues converting that value into a neutral usage snapshot, Duckboard
continues replacing its stored numerator rather than accumulating updates, and the
selected model's catalog context window remains the denominator.

Codex remains responsible for the meaning of its supplied `totalTokens`; the harness does
not recompute it from cached-input, output, or reasoning fields.

## Verification

Tests cover the seams owned by the integration:

```
| Test seam | Coverage |
| --- | --- |
| Repository policy discovery | Direct `.git/` and `.jj/` directories are included in stable order; absent directories and `.git` file indirections are excluded; roots are normalized. |
| App-server request construction | `turn/start` contains workspace-write and exactly the remembered metadata roots, including an empty-root policy. |
| Agent lifecycle | Scripted app-server tests cover new, loaded, process-hot, and respawned sessions plus discovery refresh on a later load. |
| Usage mapping | Latest-turn total wins over a much larger cumulative total; cumulative fallback and missing telemetry are covered. |
```

Tests use temporary repositories and the existing scripted JSON-RPC backend. They do not
mutate the developer's repository, invoke real Git or Jujutsu operations, require Codex
authentication, or snapshot unrelated app-server request fields.
