# Permission-grant auto-approval shapes

Branch App Server server-requests so command/file approvals keep `decision` accept, while
`item/permissions/requestApproval` returns a grant body (echo requested profile for the
turn).

## Prerequisites

- [x] @step empty-model-catalog-when-backend-unavailable

## Context

From review finding 2
(`reviews/02-review-post-implementation-review-of-openai-codex-harness.md`).

Schema (from `codex app-server generate-json-schema`): permissions responses require
`permissions` (GrantedPermissionProfile), not `decision`. Today any method name containing
`Approval` gets `{ "decision": "accept" }`, which is wrong for
`item/permissions/requestApproval` and can stall turns. Product policy: auto-grant by
echoing requested permissions with `"scope": "turn"` (parity with `approvalPolicy: never`
/ ordinary tools auto-approved).

## Tasks

- [x] 1. Split `is_ordinary_approval_method` / `auto_allow_approval_result` in
         `crates/duckchat-codex-acp/src/codex/ask_user.rs` so
         `item/permissions/requestApproval` is not treated as a `decision` accept

- [x] 2. Pass **params** from `handle_incoming` in
         `crates/duckchat-codex-acp/src/codex/app_server.rs` into the auto-allow path; for
         permissions return `{ "permissions": <requested or {}>, "scope": "turn" }`

- [x] 3. Keep `decision: accept` (and legacy `approved` for exec/patch) for command and
         file approval methods

- [x] 4. Unit-test method classification and JSON shapes for command, file, and
         permissions approval methods

- [x] 5. @spec harness/openai-codex Ordinary tools stay auto-approved: Ordinary tool permission does not require host UI
