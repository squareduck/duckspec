# Claude agent live catalog

Discover Claude models via the Anthropic Models API inside `duckchat-claude-acp` and
advertise them on initialize, with a curated alias fallback when live discovery fails.

## Tasks

- [x] 1. Add live model discovery in `duckchat-claude-acp` using credentials available to
         the official `claude` install (`GET /v1/models`)

- [x] 2. Map API models into ACP `availableModels` (`modelId`, `name`,
         `totalContextTokens` when known)

- [x] 3. On auth/network/parse failure, advertise the curated alias fallback set
         (non-empty) instead of an empty list

- [x] 4. @spec harness/claude Agent model advertise: Successful live discovery advertises those models on initialize

- [x] 5. @spec harness/claude Agent model advertise: Failed live discovery advertises the curated alias fallback
