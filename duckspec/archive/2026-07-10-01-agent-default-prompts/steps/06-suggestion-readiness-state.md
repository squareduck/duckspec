# Suggestion readiness state

Track pending vs ready for the reply-suggestion oneshot so empty Enter and Tab never arm a
list that can still change under the user.

## Prerequisites

- [x] @step effective-list-is-oneshot-only

## Context

Addresses the major readiness finding in `reviews/01-post-implementation-review.md`. While
a oneshot is outstanding for the current `default_prompts_gen`, suggestions are pending:
no list arming, empty submit is a no-op. On settle (ok or err) with matching gen → ready
with the effective list (possibly empty). Superseded gen results are ignored (already
partially true — keep and cover). Pure helpers in `default_prompts.rs` for “may
empty-submit / may cycle / presentation mode” keep UI thin; wire `AgentSession` +
`TurnComplete` / `DefaultPromptsReady` / send clear paths.

## Tasks

- [x] 1. Add pending/ready state on `AgentSession` (explicit flag or enum alongside
         `default_prompts_gen`); set pending when a oneshot is spawned; clear/supersede on
         new turn send

- [x] 2. On `DefaultPromptsReady`: matching gen → store list, mark ready; mismatch →
         ignore; on error → ready with empty list

- [x] 3. Gate empty-input submit and Tab/Shift-Tab cycle on ready + non-empty list
         (pending ⇒ empty submit no-op, no cycle)

- [x] 4. @spec chat/default-prompts Suggestion readiness: Empty submit is a no-op while pending

- [x] 5. @spec chat/default-prompts Suggestion readiness: Ready after settle arms the effective list

- [x] 6. @spec chat/default-prompts Suggestion readiness: Superseded generation does not arm the list
