# Extract shared ACP client

Lift the Grok ACP client into a harness-neutral `duckchat::acp` module with `AgentLaunch`,
and rewire `GrokProvider` onto shared main/oneshot runtimes without changing Grok
behavior.

## Tasks

- [x] 1. Create `crates/duckchat/src/acp/` (`launch.rs`, `turn.rs`, `event.rs`,
         `runtime.rs`) and export it from `lib.rs`.

- [x] 2. Introduce `AgentLaunch` (spawn factory for final argv) and make
         `AcpTurn::spawn_with` use the launch as-is (no harness-hardcoded flags inside the
         client).

- [x] 3. Move JSON-RPC transport, initialize/open/prompt, session-not-found mapping, and
         cancel from `grok/acp.rs` into `acp/turn.rs`; move `map_update` into
         `acp/event.rs`.

- [x] 4. Move `GrokMainRuntime` / `GrokOneshotRuntime` into shared `AcpMainRuntime` /
         `AcpOneshotRuntime` parameterized by `AgentLaunch` (and optional prompt knobs
         such as reasoning).

- [x] 5. Rewire `GrokProvider` to build a Grok `AgentLaunch`
         (`grok --no-ask-user
             agent --always-approve stdio` via login-shell wrap) and
         open shared runtimes; slim `grok/` to provider + spawn + Grok-only helpers.

- [x] 6. Migrate existing Grok unit/integration tests to the new module paths; keep the
         suite green for Grok.

- [x] 7. @spec harness/acp-client Launch-parameterized agent process: The client spawns the launch-supplied agent command

- [x] 8. @spec harness/acp-client Launch-parameterized agent process: A second turn on a hot main path reuses the agent process

- [x] 9. @spec harness/acp-client Launch-parameterized agent process: After cancel, a later turn may spawn again and resume a prior session id

- [x] 10. @spec harness/acp-client Session open and resume: A turn without a prior session id opens a new session and surfaces the id

- [x] 11. @spec harness/acp-client Session open and resume: A turn with a prior session id resumes that id

- [x] 12. @spec harness/acp-client Session open and resume: A failed load of a missing session surfaces session-not-found

- [x] 13. @spec harness/acp-client Profile event translation: Assistant text and reasoning surface on distinct channels

- [x] 14. @spec harness/acp-client Profile event translation: A tool call surfaces as a use then a matching result

- [x] 15. @spec harness/acp-client Profile event translation: A usage update carries used tokens and the model's context window

- [x] 16. @spec harness/grok Native Grok agent launch: A Grok turn spawns the native grok ACP agent
