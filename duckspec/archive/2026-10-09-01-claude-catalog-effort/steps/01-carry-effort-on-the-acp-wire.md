# Carry effort on the ACP wire

The shared client reads optional effort and catalog status from initialize, and sends a
turn's effort level as `effort`.

## Tasks

- [x] 1. Add optional per-model effort and catalog status to initialize parsing in
         `crates/duckchat/src/acp/turn.rs`. A missing catalog object stays ok with no
         expiry.

- [x] 2. Add `effort` on `TurnRequest` and copy it to `session/prompt` as `effort`,
         leaving `reasoningEffort` for reasoning mode.

- [x] 3. @spec harness/acp-client Initialize handshake metadata: Advertised effort is carried when the model sends it

- [x] 4. @spec harness/acp-client Initialize handshake metadata: Catalog status is optional handshake metadata

- [x] 5. @spec harness/acp-client Prompt effort: Effort and reasoning mode use different prompt fields
