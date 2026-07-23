# Correct Codex context usage

Prefer latest-turn token totals in Codex profile normalization while retaining cumulative
compatibility fallback and missing-data behavior.

## Tasks

- [x] 1. Change Codex token-usage normalization to select latest-turn total before
         cumulative thread total and emit nothing when both are absent

- [x] 2. @spec harness/openai-codex Profile-compatible event emission: Token telemetry surfaces as usage with total tokens

- [x] 3. @spec harness/openai-codex Profile-compatible event emission: Cumulative token telemetry is used when latest-turn usage is absent

- [x] 4. @spec harness/openai-codex Profile-compatible event emission: Missing token totals emit no usage update
