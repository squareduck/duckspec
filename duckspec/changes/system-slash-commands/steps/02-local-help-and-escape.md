# Local /help and // escape

Route bare system submits to local handlers (v1 `/help`) and bare `//name` to the agent
with prompt `/name` while keeping the typed user text.

## Prerequisites

- [x] @step kinded-catalog-and-discovery-cleanup

## Tasks

- [x] 1. Add pure `parse_submit_slash` (local system bare name vs `//` escape vs normal
         agent text) next to the send path

- [x] 2. Implement pure `build_system_help_body(catalog, harness_id)` with fixed prefix
         (running `/help` + teach `//help`) and non-empty kind sections from the live
         catalog

- [x] 3. Implement `run_system_help`: user message + system message, clear composer,
         persist; no streaming, priming, or selection-attachment consumption

- [x] 4. Split send so User bubble text and `TurnRequest` prompt can differ; wire
         `SendPressed` / empty-submit through `parse_submit_slash`

- [x] 5. @spec chat/slash-commands Local system submit: Bare /help does not start an agent turn

- [x] 6. @spec chat/slash-commands Local system submit: Bare /help records user then system messages

- [x] 7. @spec chat/slash-commands Local system submit: Local /help leaves selection attachments intact

- [x] 8. @spec chat/slash-commands Local system submit: System reply prefix names the command and teaches //help

- [x] 9. @spec chat/slash-commands Local system submit: Help body lists non-empty kind sections from the live catalog

- [x] 10. @spec chat/slash-commands Double-slash agent escape: Bare //help is an agent turn with prompt /help

- [x] 11. @spec chat/slash-commands Double-slash agent escape: Escape keeps typed //help as the user message text
