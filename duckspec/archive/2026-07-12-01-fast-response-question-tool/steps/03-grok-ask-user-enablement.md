# Grok ask-user enablement

Enable Grok structured questions on the main launch and map `x.ai/ask_user_question` to
the shared user-choice path with accepted / skip-interview responses.

## Prerequisites

- [x] @step acp-mid-turn-user-choice

## Tasks

- [x] 1. Drop `--no-ask-user` from `grok_agent_launch` while keeping `--always-approve`
         (update launch tests)

- [x] 2. Decode `x.ai/ask_user_question` params into neutral `UserChoiceRequest` options
         (v1: sequential single-select labels)

- [x] 3. Encode host `Selected` as `AskUserQuestionExtResponse::Accepted` with answers;
         encode `Cancelled` as `SkipInterview`

- [x] 4. @spec harness/grok Structured questions enabled: Main launch does not pass no-ask-user

- [x] 5. @spec harness/grok Structured questions enabled: Main launch still auto-approves tool execution

- [x] 6. @spec harness/grok Question wire mapping: An ask-user extension request is exposed as a host user choice

- [x] 7. @spec harness/grok Question wire mapping: A host selection completes with an accepted questionnaire response

- [x] 8. @spec harness/grok Question wire mapping: A host cancel completes with a skip-interview response
