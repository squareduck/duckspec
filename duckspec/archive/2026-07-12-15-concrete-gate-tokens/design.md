# Concrete gate tokens - Design

Stock style and templates emit decision-named gate tokens; `/ds-spec` restores a separate
map stage with REMOVE. No runtime or meta-card parser changes.

## Approach

```
┌─────────────────────────────────────────────────────────────┐
│  style.md (shared rule)                                     │
│  · send tokens name the decision                            │
│  · decision tokens: no reason                               │
│  · slash commands: keep short reason                        │
└───────────────────────────┬─────────────────────────────────┘
                            │ examples + Write gate rule
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
   templates/*          template-and-       (runtime unchanged)
   write-gate/next      schema-authoring    meta_card: full first
   + spec map restore   one durable line    code span = send
```

Implementation is **embedded stock content** under `crates/duckspec/content/` (and a small
codex note). Duckboard already chips and sends the full first backtick span;
`NextAction.reason` stays optional and unused for ghost/send.

```
after thrash/cancel (resync may or may not fire)
user sends: confirm caps/archive/browse
            └──────── names the gate ────────┘
agent matches path → write that cap (not re-map)
```

## Style (`content/schemas/style.md`)

Update the `next` meta card and Write gate sections:

- **Send tokens** name the decision or stage action (`confirm proposal`, `confirm map`,
  `create change <name>`). Bare `confirm` / `reject` are not the stock pattern.

- **Reasons:** omit for decision tokens; keep a short UI reason for slash-command handoffs
  only.

- Replace write-gate and handoff examples accordingly:

```markdown
> **next**
>
> `confirm proposal`
> `reject proposal`
```

```markdown
> **next**
>
> `/ds-design`  design the approach
> `/ds-spec`    write specs
```

Prose that says “includes `confirm` and usually `reject`” becomes “includes a
decision-named confirm token and usually a decision-named reject.”

## Token vocabulary (all write-gate templates)

```
| Stage                    | Confirm send              | Reject send (when offered) |
|--------------------------|---------------------------|----------------------------|
| explore / backfill create| `create change <name>`    | `reject change` (backfill) |
| explore project          | `write project.md`        | — (freeform)               |
| propose                  | `confirm proposal`        | — or freeform              |
| design                   | `confirm design`          | `reject design`            |
| spec map                 | `confirm map`             | — (freeform rework)        |
| spec CREATE / UPDATE     | `confirm <path>`          | — (freeform)               |
| spec REMOVE              | `confirm remove <path>`   | —                          |
| step                     | `confirm steps`           | `reject steps`             |
| review                   | `confirm review`          | `reject review`            |
| followup                 | `confirm followup`        | `reject followup`          |
| codex                    | `confirm entry`           | `reject entry`             |
| archive                  | `confirm archive`         | `reject archive`           |
```

Path is the capability path as in the outline H1. No `create`/`update` qualifier on
confirm — the outline preview already carries CREATE/UPDATE; only REMOVE needs a distinct
token.

Templates also update instructional prose that says “wait for `confirm`” to name the
concrete token.

## Spec template restore (`content/templates/spec.md`)

Return to the **original multi-step flow** (pre-split, pre-map+first), plus REMOVE from
confirm-gate:

```
1. Map (CREATE / UPDATE / REMOVE paths, one-line each)
2. Wait — next card: `confirm map` only
3. Per map item, in order:
   outline write gate → wait `confirm <path>` or `confirm remove <path>`
   → expand, write, format, check → next item
4. Handoff (slash reasons kept)
```

Not: map+first outline under one confirm. Not: split outline/write turns.

Map gate shape:

```markdown
# CREATE <path>
…

# UPDATE <path>
…

# REMOVE <path>
…

> **next**
>
> `confirm map`
```

Per-cap and REMOVE gates use the vocabulary table; no reason text on those lines.

## Codex (`duckspec/codex/template-and-schema-authoring.md`)

One durable principle: write-gate send tokens name the decision; reasons only on
slash-command handoffs. Keeps future templates from reintroducing bare `confirm`.

## Impact

- **Stock content only:** `style.md`, all templates with write gates, optional codex line

- **`ds` rebuild/reinstall** required (content is compile-time embedded)

- **No** duckboard / meta-card / cancel-resync code

- **No** new capabilities; `chat/meta-cards` already allows multi-word send text

- Unit tests that hardcode bare `confirm` with a reason remain valid (optional-reason
  path); no required test churn unless examples should track stock wording

## Decisions

- **Path-only confirm for CREATE/UPDATE** — `confirm <path>`, not `confirm create <path>`.
  Alternatives: always include verb (noisier; rejected for outline gates). REMOVE keeps
  the verb because the action differs.

- **Restore separate map stage** — original UX; map+first and split-turn were loop
  mitigations that failed. Alternative: keep map+first with only token renames (rejected:
  worse authoring UX without proven loop benefit).

- **No parser/UI change** — reasons stay parseable but templates stop emitting them on
  decision tokens; ghost continues to show `send` only.

- **Codex note yes** — small durable rule so style is not the only memory.

## Risks

- **Agents still emit bare `confirm`** → style + every template example + instructional
  “wait for …” lines all use concrete tokens; codex reinforces.

- **Long path tokens crowd chips** → accept; full path is the disambiguator. Paths are
  already short kebab segments.

- **Users type bare `confirm` from habit** → agents should treat freeform “yes/confirm” as
  soft intent only when a single gate is open; templates still require the concrete token
  in the `next` card (chip path is correct by construction).
