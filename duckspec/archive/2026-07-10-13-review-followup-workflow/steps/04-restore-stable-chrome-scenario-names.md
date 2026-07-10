# Restore stable chrome scenario names

Fix project-audit breakage from renaming obvious-bubble scenarios: keep stable titles,
body-only updates, and matching `@spec` strings.

## Prerequisites

- [x] @step lifecycle-dual-critique-chrome

## Context

Addresses finding 1 in `reviews/03-review-post-implementation-dual-critique.md`. Prefer
stable scenario titles over renames so archived checked `@spec` tasks keep resolving. Do
not edit archived change files unless a body-only approach still leaves audit red after
retargeting.

Stable names to restore (match top-level / archive `@spec` text exactly):

- `Open steps with review yield apply only with gate`
- `No open steps with review yield step then spec then archive with gate`

## Tasks

- [x] 1. In
         `duckspec/changes/review-followup-workflow/caps/chat/obvious-bubble/spec.delta.md`,
         remove the two `=` rename ops; ensure dual-critique THEN lists (apply + review +
         followup / rework + critique + archive) live under `~` replaces of the **stable**
         scenario titles above (bodies may still describe followup; titles stay stable)

- [x] 2. In `crates/duckboard/src/area/change.rs`, retarget `@spec` markers and any test
         names that used the renamed titles back to the stable scenario strings; keep
         assertion vectors that include `/ds-followup`

- [x] 3. Update this change's `steps/02-lifecycle-dual-critique-chrome.md` checked `@spec`
         task lines to the same stable scenario names (single unbroken lines)

- [x] 4. @spec chat/obvious-bubble Chrome composition: Open steps with review yield apply only with gate

- [x] 5. @spec chat/obvious-bubble Chrome composition: No open steps with review yield step then spec then archive with gate

- [x] 6. Run `cargo test -p duckboard` for chrome composition tests and full `ds audit`
         (no change filter) until the two prior project audit errors are gone
