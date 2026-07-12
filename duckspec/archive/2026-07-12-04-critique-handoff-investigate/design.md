# Critique handoff investigate - Design

Content-only change to stock agent templates and the shared `style` schema: post-write
handoff ranking for review/followup, plus drop noop `ignore` from taught `next` tokens.

## Approach

No product code, parsers, or duckboard changes. Bare tokens already send as freeform
composer text (`chat/meta-cards`); `investigate` needs no new chrome.

```
  review|followup write succeeds
              │
              ▼
     cold /ds-spec or /ds-step
     could act without re-deriving?
          /              \
        no                yes
         │                 │
         ▼                 ▼
   next: `investigate`   next: ranked stages
   (optional archive)    /ds-spec before /ds-step
                         when behavior/invariants;
                         /ds-step alone for pure rework;
                         /ds-archive if freeze-ready
         │
         └─ nothing useful → omit next meta card
```

Edit three stock files under `crates/duckspec/content/` (embedded by `ds`; shipped via
existing `cli/stock-content` path). Keep Handoff altitude thin — bullets + one readiness
sentence, not a decision tree.

## Review and followup templates

Files: `templates/review.md`, `templates/followup.md`.

Replace **Handoff** only (Role / Instructions / Write gate unchanged). Shared wording for
both stages so ranking stays consistent:

```markdown
## Handoff

After a clean write, emit a `next` meta card only when there is a useful
action (≤3 lines, short UI labels, rank order). Include only lines that apply:

- `investigate` - dig further in chat
  (fix path still unclear; cold /ds-spec or /ds-step could not act yet)
- `/ds-spec` - write specs
  (primary when behavior or invariants change; pair `/ds-step` second to skip)
- `/ds-step` - plan implementation
  (specs already cover it, or pure rework)
- `/ds-archive` - archive change
  (ready to freeze)

Offer `/ds-spec` or `/ds-step` only when a cold run could act without
re-deriving the conversation; otherwise prefer `investigate`. Omit the
card when none of the above apply. Do not auto-start.
```

Drop `ignore` and the “always emit” wording. Leave “user may discuss / in-place fixes
after the document exists” only if it still fits in one short closing line; otherwise fold
into “omit when nothing to offer.”

Do **not** touch schema Structure/examples for review or followup (disk `→ next` cells may
still say whatever the agent wrote; historical `ignore` rows stay as prose history).

## Style schema

File: `schemas/style.md`.

In the `next` meta card send-token example list, remove `` `ignore` `` so style does not
teach a noop chip:

```markdown
# before
alike: `/ds-step`, `confirm`, `reject`, `revise`, `Create change`, `ignore`, …

# after
alike: `/ds-step`, `confirm`, `reject`, `revise`, `Create change`, …
```

Optional one-word note is unnecessary — existing “Omit the entire `next` meta card when
there is nothing to offer” already covers freeform-only continuation.

## Verification

- `ds template followup` / `ds template review` show new Handoff; no `` `ignore` ``
- `ds schema style` example list has no `` `ignore` ``
- Existing `cli/stock-content` tests remain green (they do not snapshot handoff text)

No new unit/integration tests required unless we later encode handoff text in fixtures
(out of scope).

## Impact

- Agents using stock templates get new post-write guidance after rebuild/reinstall of `ds`
  content

- Duckboard chips: any send token in a trailing `next` card already works; `investigate`
  needs no client change

- No API, migration, or cap tree layout change

## Decisions

- **Template-only, no new caps** — Handoff is agent process guidance; `cli/stock-content`
  already owns shipping. Alternatives: new “critique-handoff” cap (rejected: bloat for
  prompt text).

- **Identical Handoff text in review and followup** — same ranking problem. Alternatives:
  stage-specific wording (rejected: drift).

- **Leave review/followup artifact schemas alone** — proposal non-goal on historical disk
  labels. Alternatives: ban `ignore` as `→ next` cell (rejected: not meta-card chrome; not
  worth schema touch).

- **Bare token `investigate`** — clickable continue-digging path. Alternatives: omit
  stages only (rejected: less discoverable).

## Risks

- **Models still default to `/ds-step` from old habits or schema examples** → readiness +
  ranking bullets first; revisit examples only if seen in practice.

- **`investigate` chip sends a bare word with no slash command** → intended; agent stays
  in review/followup conversation and digs further (not a stage start).

## Open questions

None — readiness bar, ranking, `investigate`, and drop-`ignore` settled in
explore/propose.
