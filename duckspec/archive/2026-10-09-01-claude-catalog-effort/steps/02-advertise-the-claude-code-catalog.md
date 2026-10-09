# Advertise the Claude Code catalog

The Claude agent advertises the filtered public catalog, and the host lists that set.

## Prerequisites

- [x] @step carry-effort-on-the-acp-wire

## Context

The client already parses handshake effort and catalog status. Emit that wire shape; do
not add a second parser.

- `AcpModel.effort` is `Option<ModelEffort>` (`default_level`, `levels` in
  `crates/duckchat/src/provider.rs`). It comes from the model object's `_meta.effort`
  `{ "default", "levels" }`. A missing or incomplete object is no effort. `ModelInfo` does
  not carry effort yet.

- `InitResult.catalog` is `Ok { expires_at }` or `Failed`. A missing `modelState.catalog`
  object is ok with no expiry. The object is
  `{ "status": "ok" | "failed", "expiresAt": "<rfc3339>" }`, and `expiresAt` is omitted
  when the document has none. `list_models` still returns only models.

## Tasks

- [x] 1. Fetch the Claude Code catalog with no auth during initialize in
         `crates/duckchat-claude-acp/src/models.rs`, and remove the alias fallback.

- [x] 2. Admit main-section rows by the binary version, map name, window, and effort, and
         report catalog status and expiry. A failed fetch still initializes and advertises
         no models.

- [x] 3. Carry effort through `ModelInfo` and `list_models` in
         `crates/duckchat/src/claude_code.rs`. Retire the host empty-on-failure test.
         Slice retention is step 04.

- [x] 4. @spec harness/claude Agent model advertise: Admitted catalog rows carry name, window, and effort

- [x] 5. @spec harness/claude Agent model advertise: An unreadable binary version drops rows that require a minimum

- [x] 6. @spec harness/claude Agent model advertise: A failed catalog fetch advertises no models

- [x] 7. @spec harness/claude Agent model advertise: Catalog expiry is reported only when the document has one

- [x] 8. @spec harness/claude Model discovery: Listed models follow the agent advertise set

## Outcomes

The version probe uses the same argv as the spawned `claude` (`DUCKCHAT_CLAUDE_BIN`, or
the login-shell wrap). A failed, timed-out, or unparseable `--version` drops rows that
require a minimum. Effort is advertised only when `thinking.type` is `effort`,
`runtime.effort_levels` is a non-empty list of ids, and `runtime.default_effort` is set.
The client drops an incomplete effort object. Claude `ModelInfo.effort` now carries that
scale.
