# Post-implementation review: agent default prompts

Reviewed `agent-default-prompts` end-to-end (proposal → design → caps → code). Parse/merge
logic is solid and well-tested, but the composer UI has a critical auto-grow regression
and the oneshot “heuristic first, then swap” path can change the active Enter target under
the user — both block acceptance.

## Scope

Post-implementation: proposal, design, `caps/chat/default-prompts`, steps 01–03, and the
work under `crates/duckchat` (parse + provider oneshots) and `crates/duckboard`
(`default_prompts`, session wire-up, `agent_chat` composer).

## Findings

### Stack base layer caps chat input auto-grow — soundness/critical

`crates/duckboard/src/widget/agent_chat.rs` builds the empty-input chrome as
`stack![prompt_list_body, input]`. In iced 0.14, the **first** child is the base layer and
**dictates the stack’s intrinsic size**; later layers are laid out with that size as their
max (`iced_widget::stack`: “The first Element dictates the intrinsic Size”).

So whenever any effective default prompt exists (almost always, once `obvious_command` is
set), the transparent `TextEdit` is height-limited to the prompt list (~1–4 rows).
Multi-line typing scrolls inside that cap instead of growing up to `CHAT_INPUT_MAX_ROWS`.
When the list is empty the stack is skipped and grow works — the regression is tied to
this change’s UI.

The design mockup shows a normal empty editor **above** a separate list strip, not a ghost
stack under a transparent field. The overlay was an implementation choice; it breaks a
load-bearing composer behavior.

**Action:** Restore grow. Prefer the design’s column layout (list as a sibling below the
editor when empty), or keep a stack only if the **input** is the base layer and the list
is `push_under` **without** owning height — and still grow the outer composer to
`max(list_height, editor_height)` so empty-state list rows remain visible. Verify
multi-line paste/type with and without a non-empty default list.

### Active default can change under Enter while oneshot is in flight — soundness/major

After `TurnComplete`, the code clears agent suggestions and immediately shows the
heuristic-only effective list; when `DefaultPromptsReady` arrives it injects 0–3 agent
replies **in front** of the heuristic (`main.rs` TurnComplete / `DefaultPromptsReady`,
`effective_prompts`). `default_prompt_idx` stays at `0` across that transition, so the
active Enter target silently flips from `/heuristic` to the first agent reply. A user who
reads the list and presses Enter as the oneshot lands can send the wrong prompt — exactly
the failure reported in review.

The design’s risk section accepted this tradeoff explicitly (“heuristic-only list until
ready; no spinner required”). That tradeoff does not hold in practice: latency is long
enough for a deliberate Enter, and prepending agent rows is the worst shape for a stable
active index.

**Action:** Treat oneshot completion as a readiness gate for **showing and arming**
auto-suggest options:

1. Track pending vs ready (gen + explicit pending/ready state is fine).

2. While pending, do not present a multi-option list that will be replaced; show a loading
   indicator instead (user request). Decide product-wise whether Enter during pending is
   no-op, still-heuristic-only, or disabled — but **do not** let the labeled active entry
   change meaning mid-gesture.

3. When ready (success or failure), show the final effective list once.

4. Update design + `caps/chat/default-prompts` so readiness/loading is specified, not only
   “not ready → heuristic-only”.

### Ghost-stack empty chrome diverges from the designed list strip — fidelity/minor

Design “Composer UI” draws an empty `TextEdit` and a **separate** list under it with an
active marker and optional “Enter · Tab …” hint. The code instead stacks a transparent
editor over a rotated list so the caret sits on the first row. That divergence is what
forced the stack base-layer choice above; even after the grow fix, prefer the simpler
sibling layout unless the ghost caret is a deliberate product requirement worth extra
layout code.

**Action:** Align presentation with the design strip (or amend the design if the ghost
list is intentional and re-specified with safe layout rules).

## Verdict

Not ready. Parse, merge, gen-matched oneshot, and empty-submit/cycle pure helpers
faithfully implement the data path and are covered by unit tests. Acceptance is blocked by
(1) the composer no longer auto-growing whenever defaults exist — a regression in everyday
chat typing — and (2) the ready-race that can send a different default than the one the
user saw. Fix grow via layout (`/ds-step`); specify and implement readiness + loading
before arming multi-option defaults (`/ds-spec`, then step).
