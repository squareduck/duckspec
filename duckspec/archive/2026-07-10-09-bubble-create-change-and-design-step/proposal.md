# Obvious chrome for create-change and design step

Nonempty exploration gets a Commit-style green ⌘↩ **Create change** affirm. Pre-step
lifecycle and the Confirm gate are tightened to match template handoffs: design offers
spec then step; caps without steps drop re-entry `/ds-spec`; implementation and
review-rework phases drop Confirm/Reject and use a review-aware apply/step ladder.

## Motivation

After a real explore chat, chrome goes blank just when the natural next move is to create
a change. Mid-lifecycle, Confirm/Reject stays up through apply even though handoffs are
slash-commands, and `/ds-spec` reappears after caps already exist even though one
`/ds-spec` session walks the whole proposal scope. After a review, chrome still pushes
archive/review instead of the review template’s rework path (`/ds-step` / `/ds-spec`, with
archive still available when ready to finish).

## Scope

```
caps/
├── chat/
│   └── obvious-bubble/     ← MODIFIED
└── session/
    └── scope/              ← MODIFIED (next stage may follow review-aware ladder)
```

### New capabilities

- None.

### Modified capabilities

- `chat/obvious-bubble` — composition and gate:

  - **Exploration:** empty → `/ds-explore`; nonempty → affirm `Create change` only (no
    Reject). New `Affirm::CreateChange`, send text literal `Create change`.

  - **Design, no caps, no review:** `/ds-spec`, then `/ds-step`.

  - **Caps, no steps, no review:** `/ds-step`, then `/ds-archive` (drop `/ds-spec`).

  - **Open steps, no review:** `/ds-apply`, then `/ds-review`; no Confirm/Reject.

  - **Open steps + review:** `/ds-apply` only; Confirm + Reject (nonempty session).

  - **No open steps + review** (no steps or all complete): `/ds-step`, `/ds-spec`,
    `/ds-archive`; Confirm + Reject (nonempty session).

  - **All complete, no review:** `/ds-archive`, then `/ds-review`; no Confirm/Reject.

  - **Confirm+Reject gate:** nonempty session and (at least one review **or** no steps on
    disk). Empty session omits the gate. Open steps without a review stay lifecycle-only.
    Commit path unchanged.

- `session/scope` — suggested next stage follows the same first lifecycle option as
  chrome, including the review-aware arms. Reported step progress remains independent of
  whether a review file exists. Orientation still reports the current review path when
  present.

### Out of scope

- Promotion / `ds create change` mechanics
- Oneshot default-prompts / composer list
- Parsing review verdict text (presence of any review file is the signal)
- Empty-exploration beyond `/ds-explore` only
- New capabilities

## Impact

```
change_scope_facts (priority):
  open steps + review     → [ds-apply]
  open steps              → [ds-apply, ds-review]
  no open steps + review  → [ds-step, ds-spec, ds-archive]
  all done, no review     → [ds-archive, ds-review]
  caps, no steps          → [ds-step, ds-archive]
  design, no caps         → [ds-spec, ds-step]
  …

build_obvious_chrome gate:
  Confirm+Reject ⇔ nonempty && (has_review || no steps)
```

Touches `obvious_bubble.rs`, `area/change.rs` facts/chrome, and matching scenarios under
`chat/obvious-bubble` and `session/scope`.
