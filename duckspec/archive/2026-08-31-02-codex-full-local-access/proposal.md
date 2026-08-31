# Codex full local access

Let Codex agents launched by Duckboard operate with the same local access as Duckboard's
OS user, without approval prompts or Codex filesystem sandbox restrictions.

The current Codex integration uses an approval-free workspace sandbox. It can modify
project files and direct version-control metadata, but the sandbox can prevent access to
databases, local services, sockets, external metadata stores, and other resources needed
to work on the project.

Codex turns should instead run with unrestricted local access. This allows the agent to
use every project dependency available to Duckboard's user, including databases and
services outside the repository directory.

This is intentionally a machine-level permission boundary, not a repository-scoped one.
Codex may access unrelated files, credentials, processes, networks, and services available
to the user. Duckboard should describe the behavior as full local access so users are not
given a false impression of project-only containment.

Project instructions and agent behavior remain responsible for limiting actions to the
requested work. They do not provide operating-system isolation and must not be presented
as a security boundary.
