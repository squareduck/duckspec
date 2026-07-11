# Grok ask-user wire fix

Fix Grok null path: match `_x.ai/ask_user_question` and encode `outcome`-tagged accepted /
skip_interview responses (live-proven).

## Context

Live ACP capture: method is **`_x.ai/ask_user_question`** (leading underscore), not
`x.ai/ask_user_question`. Classifier miss → `result: null` → tool fails, no chips.

Working response shapes (probed live):

```
select:  { "outcome": "accepted", "answers": { "<question>": "<label>" }, "partial_answers": null }
cancel:  { "outcome": "skip_interview" }
```

Allowed `outcome` variants: `accepted`, `chat_about_this`, `skip_interview`, `cancelled`.
See followup `reviews/01-followup-live-question-hangs.md` issue 2.

## Tasks

- [x] 1. Classify `_x.ai/ask_user_question` (and `x.ai/ask_user_question` alias) as user
         choice in `crates/duckchat/src/acp/turn.rs`

- [x] 2. Encode select as `{ outcome: "accepted", answers, partial_answers }`; cancel as
         `{ outcome: "skip_interview" }` in `crates/duckchat/src/acp/ask_user.rs`

- [x] 3. Assert encode/classify against live capture method name and working JSON shapes

- [x] 4. @spec harness/grok Question wire mapping: An ask-user extension request is exposed as a host user choice

- [x] 5. @spec harness/grok Question wire mapping: A host selection completes with an accepted questionnaire response

- [x] 6. @spec harness/grok Question wire mapping: A host cancel completes with a skip-interview response
