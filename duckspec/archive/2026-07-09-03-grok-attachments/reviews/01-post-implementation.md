# Post-implementation review: Grok session attachments

Reviewed `grok-attachments` end-to-end against proposal, design, caps, and the duckchat
implementation. The plan and code are sound and faithful, but the primary integration
boundary — multi-block content on `session/prompt` — is not locked by a test the design
already called for.

## Scope

Post-implementation: full chain down to code.

- `duckspec/changes/grok-attachments/proposal.md`, `design.md`

- Cap deltas under `caps/harness/grok/`

- Steps `01-extract-shared-attach-walk`, `02-grok-multi-block-prompts`

- Code: `crates/duckchat/src/attach.rs`, `claude_code/run.rs`, `grok.rs`, `grok/acp.rs`,
  `lib.rs`

## Findings

### Multi-block `session/prompt` never locked on the wire — quality/major

The design testing table required a Grok ACP check that `prompt` params carry a
multi-block `prompt` array when attachments are present. What landed is only
`assemble_content` unit tests in `crates/duckchat/src/grok.rs` (the four `@spec` cases).
Those lock encode shape (`mimeType` / `data`, interleaved text) but never drive
`AcpTurn::prompt` / `prompt_events`.

`AcpTurn::prompt` is a thin `"prompt": content` assignment today
(`crates/duckchat/src/grok/acp.rs:175-178`), and `run_turn` does wire `assemble_content`
through (`grok.rs:168-178`). Both are correct on inspection — and both can regress
independently of the `@spec` tests: re-wrapping content as a single text block, or
stopping to call `assemble_content`, would keep every current test green while restoring
the dead-markdown bug this change exists to fix.

The scripted peer harness in `grok/acp.rs` already captures written JSON for `session/new`
and `session/load`. Extend it: call `prompt` with a multi-block slice (text + image +
text), assert the captured `session/prompt` params have `prompt` as that array with ACP
image fields — not Anthropic `source`. That is the design's missing row and the only
load-bearing gap before acceptance.

### System-fold prefix untested through multi-block assembly — quality/minor

`fold_system_and_prompt` is the renamed body of the old `assemble_prompt` and is now the
first stage of `assemble_content` (`grok.rs:236-251`). No test builds a `TurnRequest` with
a non-empty `system_additions` plus an attach marker and checks that the leading text
block still carries the folded system text ahead of the image. Attachment `@spec` cases
all use empty system additions; a broken fold would drop scope blurbs while leaving
attachment tests green. Add one `assemble_content` case with a system addition and a
resolved image.

## Verdict

Well-conceived and well-made: the shared crate-private `attach` walk is the right
boundary, Claude/Grok dual encode matches the design, ACP image shape (`mimeType`/`data`)
matches the protocol, and the four requirement scenarios are covered at the assembly
layer. Capability negotiation stays correctly deferred.

Not ready to accept until the multi-block ACP wire assertion lands — that is the change's
primary integration surface and the one test the design asked for that did not ship. The
system-fold case is cheap polish on top.
