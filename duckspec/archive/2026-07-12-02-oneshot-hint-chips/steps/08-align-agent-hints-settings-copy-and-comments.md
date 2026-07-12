# Align agent-hints settings copy and comments

Rewrite settings UI strings and stale under-input comments so agent input hints match
chip-based oneshot suggestions (review findings 1–2).

## Context

Review `02-review-oneshot-hint-chips-post-implementation`: settings still describe
under-input Cmd-Enter; config/session comments lag the chip product. Finding 3 (clear
before send) is ignored.

## Tasks

- [x] 1. In `crates/duckboard/src/area/settings.rs`, rewrite the Chat section description
         and agent-hints help text: settled freeform reply chips when eligible (⌘n /
         click), not under the composer; omit when a next-action ghost is present; default
         off; no Cmd-Enter oneshot language

- [x] 2. In `crates/duckboard/src/config.rs`, update comments on `chat` and
         `agent_input_hints` to describe oneshot chip hints (not under-input)

- [x] 3. In `crates/duckboard/src/area/interaction.rs`, update the `agent_default_prompts`
         field comment (and any adjacent oneshot comments still saying under-input chrome)

- [x] 4. Grep duckboard UI strings/comments for leftover “under-input” / “Cmd-Enter”
         agent-hint wording; fix stragglers that refer to the removed surface
