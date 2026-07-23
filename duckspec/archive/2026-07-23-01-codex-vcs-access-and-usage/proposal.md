# Codex VCS access and accurate usage

Let Codex use Git and Jujutsu normally inside the active repository, with duckspec
workflow instructions governing when version-control operations are appropriate, and make
its context meter represent active context rather than cumulative token consumption.

The current Codex sandbox permits workspace file edits but protects repository metadata.
This prevents Git operations that write `.git` and also breaks colocated Jujutsu, which
uses the Git object store and may need to write while running commands that appear
observational, such as `jj status`.

Codex should have repository-local write access to the metadata needed by both
version-control systems:

```
repository
├── working files   writable
├── .git            writable
└── .jj             writable
```

This access should remain effective for new sessions, resumed sessions, and turns after
the Codex backend restarts. It must not broaden filesystem access outside the repository.

Duckspec workflow and project instructions such as `AGENTS.md` remain the authority for
when Codex may commit or perform destructive version-control operations. The sandbox
supplies the capability to use Git and Jujutsu; it does not duplicate those behavioral
rules or impose a second approval workflow.

Codex token telemetry currently feeds cumulative thread consumption into Duckboard's
context meter. Repeated context across a long conversation can therefore produce totals in
the millions even though the active model context is much smaller. The meter should
instead use Codex's latest-turn usage snapshot, which represents the context involved in
the current turn, while retaining a safe fallback when that snapshot is unavailable.
