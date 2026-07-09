# Grok multi-block prompts

Encode attach segments as ACP content blocks, assemble multi-block prompts from
`TurnRequest`, and send them through `session/prompt` so Grok receives image payloads.

## Prerequisites

- [x] @step extract-shared-attach-walk

## Tasks

- [x] 1. Change `AcpTurn::prompt` and `prompt_events` in `crates/duckchat/src/grok/acp.rs`
         to take `&[serde_json::Value]` content blocks instead of a single text string;
         put that array on `params.prompt`

- [x] 2. Update title-summary and existing ACP integration tests that call `prompt` /
         `prompt_events` to pass a one-element text content array

- [x] 3. Add ACP encoding (`type`/`mimeType`/`data` for images; `type`/`text` for text)
         and `assemble_content(&TurnRequest)` in `crates/duckchat/src/grok.rs` — fold
         system_additions + prompt, then `attach::walk`, then encode

- [x] 4. Wire `GrokProvider::run_turn` to send `assemble_content(&req)` through
         `prompt_events`

- [x] 5. @spec harness/grok Prompt attachments: A resolved image attachment is sent as an ACP image block

- [x] 6. @spec harness/grok Prompt attachments: Surrounding text is preserved as text blocks

- [x] 7. @spec harness/grok Prompt attachments: A non-image attachment is represented as text

- [x] 8. @spec harness/grok Prompt attachments: An unresolved attach marker is left literal
