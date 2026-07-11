# Claude control request-id parse

Fix Claude `can_use_tool` hang: parse top-level `request_id` on `control_request` so
AskUserQuestion gets a `control_response` and can reach parent chips.

## Context

Live Claude 2.1 emits:

```
{ "type": "control_request", "request_id": "<uuid>",
  "request": { "subtype": "can_use_tool", "tool_name": "AskUserQuestion", "input": {…} } }
```

`parse_control_permission` only looked for `request_id` inside `request`, dropped the
line, never wrote `control_response` → infinite hang. Profile `tool_use` still paints the
pending tool. See followup `reviews/01-followup-live-question-hangs.md` issue 1.

## Tasks

- [x] 1. Fix `parse_control_permission` in
         `crates/duckchat-claude-acp/src/claude/ask_user.rs` to read top-level
         `request_id` (fallback nested); keep `request.input` as tool input

- [x] 2. Unit-test the real wire shape: top-level `request_id` + `subtype: can_use_tool` +
         `input` questions

- [x] 3. @spec harness/claude Mid-prompt parent choice: An AskUserQuestion request surfaces a host user choice

- [x] 4. @spec harness/claude Mid-prompt parent choice: Host selection completes with allow and answers

- [x] 5. @spec harness/claude Mid-prompt parent choice: Host cancel completes without accepting the questionnaire
