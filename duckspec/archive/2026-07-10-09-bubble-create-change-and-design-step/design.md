# Obvious chrome for create-change and design step — Design

Composition and gate rules for obvious chrome (and the shared orientation next-stage)
aligned with explore/spec/apply/review handoffs: Create change affirm, pre-step ladder
tweaks, no Confirm during implement/rework, and a review-aware lifecycle.

## Approach

One pure ladder in `change_scope_facts` drives lifecycle chips, soft-hint first option,
and orientation `next_command`. `build_obvious_chrome` maps that list to chrome and
applies a narrowed Confirm gate.

```
                    disk: steps / reviews / caps / design / …
                              │
                              ▼
                    change_scope_facts
                              │
              lifecycle_commands[]  next_command = first
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
    build_obvious_chrome                 orientation
    + gate (Confirm/Reject)              (next stage + progress)
```

Priority (first match wins):

```
open steps + has_review     → [ds-apply]
open steps                  → [ds-apply, ds-review]
no open steps + has_review  → [ds-step, ds-spec, ds-archive]
all steps complete          → [ds-archive, ds-review]
caps, no steps              → [ds-step, ds-archive]
design, no caps             → [ds-spec, ds-step]
proposal, no design         → [ds-design, ds-spec]
else                        → [ds-propose]
```

Gate (active change only, not Commit/CreateChange):

```
Confirm + Reject  ⇔  !session_empty && (has_review || steps.is_empty())
```

Review presence always restores the gate so mid-skill write approval (post-review
`/ds-step` / `/ds-spec`) has ⌘↩ Confirm without parsing assistant text.

Exploration and archived Commit stay outside this ladder.

```
caps/
├── chat/obvious-bubble/   composition + Affirm::CreateChange
└── session/scope/         next stage may follow review-aware first lifecycle
```

Code: `obvious_bubble.rs`, `area/change.rs` (`change_scope_facts`,
`build_obvious_chrome`).

## Affirm::CreateChange

```rust
pub enum Affirm {
    Confirm,
    Commit,
    CreateChange,
}

impl Affirm {
    pub fn send_text(self) -> &'static str {
        match self {
            Affirm::Confirm => "Confirm",
            Affirm::Commit => "Commit",
            Affirm::CreateChange => "Create change",
        }
    }
}
```

UI already greens any affirm on ⌘↩; no `agent_chat` special case.

## Exploration composition

Empty session → lifecycle `/ds-explore`. Nonempty → affirm `CreateChange`, empty
lifecycle, `decline: false` (Commit twin).

## Review-aware and pre-step ladder

```rust
// change_scope_facts — sketch
let has_review = current_review.is_some();
let has_steps = !change.steps.is_empty();
let all_done = has_steps && steps_done == change.steps.len();
let open = has_steps && !all_done;

if open {
    let life = if has_review { &["ds-apply"][..] } else { &["ds-apply", "ds-review"] };
    return Some(scope_facts(/* implementing */, life, …));
}
if has_review {
    // no open steps: none yet, or all complete after a review
    return Some(scope_facts(/* … */, &["ds-step", "ds-spec", "ds-archive"], …));
}
if all_done {
    return Some(scope_facts(/* … */, &["ds-archive", "ds-review"], …));
}
if !change.cap_tree.is_empty() {
    return Some(scope_facts(/* … */, &["ds-step", "ds-archive"], …));
}
if change.has_design {
    return Some(scope_facts(/* … */, &["ds-spec", "ds-step"], …));
}
// proposal / empty as today
```

## Gate in build_obvious_chrome

```rust
let has_steps = /* from project change */;
let has_review = facts.current_review.is_some();
let (affirm, decline) = if session_empty {
    (None, false)
} else if has_review || !has_steps {
    (Some(Affirm::Confirm), true)
} else {
    (None, false)
};
```

`ChangeScopeFacts` already carries `current_review`; may also expose step emptiness via
existing `step_count` / phase, or re-read the change in the builder.

## session/scope

Orientation still names progress and current review path. **Suggested next stage** is the
first lifecycle command from the same facts — so a review can change next stage when the
ladder does (e.g. all-complete + review → `ds-step`). Progress counters stay independent
of review presence alone.

Replace the rule “reviews SHALL NOT affect suggested next stage” and the scenario that
asserted identical next stage with vs without reviews. Add scenarios for review-aware next
stage and for progress unchanged by review.

## Decisions

- **Review signal = any review file** — do not parse verdict. Alternatives: parse “ready
  to archive” (rejected: fragile). Archive remains third option when no open steps +
  review.

- **Single ladder for chrome and orientation** — avoids dual sources of truth.
  Alternatives: chrome-only review awareness (rejected: orientation would lie).

- **Gate only pre-step without reviews** — Confirm is freeform early-stage, not apply.

- **Caps without steps drop ds-spec** when no review — one `/ds-spec` session covers
  scope; re-add `ds-spec` only via review rework arm.

- **Design order spec then step** — default formalize, skip-ahead to plan.

## Risks

- **Clean review still offers step/spec first** → archive is third chip, not hidden;
  acceptable without verdict parsing.

- **session/scope scenario churn** → update tests that assumed review-invariant next
  stage.

## Open questions

None.
