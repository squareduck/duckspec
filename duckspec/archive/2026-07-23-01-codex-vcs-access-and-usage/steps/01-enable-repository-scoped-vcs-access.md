# Enable repository-scoped VCS access

Discover direct repository metadata, retain access context across Codex session lifecycle
transitions, and apply an explicit sandbox policy to every turn.

## Tasks

- [x] 1. Add a repository-access value that normalizes the ACP working directory and
         discovers existing direct `.git` and `.jj` directories in stable order without
         following files or searching outside the repository

- [x] 2. Store and refresh repository access by Codex thread id during `session/new` and
         `session/load`, independently of process-hot session membership

- [x] 3. Extend app-server `turn/start` request construction and prompt orchestration to
         send workspace-write with the remembered additional writable roots on every turn

- [x] 4. @spec harness/openai-codex Repository-scoped VCS access: Direct repository metadata is writable on every turn

- [x] 5. @spec harness/openai-codex Repository-scoped VCS access: External metadata indirection is not granted

- [x] 6. @spec harness/openai-codex Repository-scoped VCS access: Resumed and restarted sessions reapply refreshed repository access

- [x] 7. @spec harness/openai-codex Repository-scoped VCS access: A rejected repository policy does not trigger a weaker retry
