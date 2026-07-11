# Stale build plaque

When duckboard is open on its own project and the on-disk package version is ahead of the
running binary, surface a quiet Update plaque with a manual redeploy recipe — never auto-
rebuild or restart.

## Motivation

While dogfooding duckboard on itself, it is easy to keep running an older binary after the
workspace version has moved on. Automatic rebuild-and-redeploy after every feature is too
dangerous (in-flight agent turns, binary replace on macOS, surprise process death). We
still want a clear, low-risk signal that this process is behind the code it is editing.

Why now: the self-host loop is already the main development path; a read-only version
check gives the signal without taking control of the process.

## Intent

- When the open project is duckboard itself, compare the binary’s baked package version to
  the workspace version declared on disk

- If disk is **ahead** of the running build, show a small clickable **Update** plaque in
  the UI; if equal, behind, unreadable, or not a self project, show nothing

- Clicking the plaque opens a short panel: close the app, then run a concrete
  `cd <project> && just install` command, with a control that copies that command to the
  clipboard

- No automatic build, install, process kill, or binary overwrite

## Non-goals

- Auto-redeploy, hot-swap, or restart after features or builds

- Detecting “ahead” via git SHA, mtime, or dirty tree (semver only)

- Bumping the workspace version as part of this change

- Update checks against GitHub releases or remote artifacts

- Replacing the install recipe (`just install`) with a different deploy path in this
  change
