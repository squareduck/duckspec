# Stuck reply-suggestion loading

Stop empty-input reply suggestions from locking forever on the loading `…` when a oneshot
never settles—or when pending is left true—so later turns can recover without restarting
the app.

## Motivation

After a non-priming turn, reply suggestions enter a pending state and the empty composer
shows a loading indicator (`…`). Typing hides that chrome by design; erasing should
restore the ready list once the oneshot settles. In practice, a hung serial oneshot—often
title summary queued ahead of reply suggestions, with no timeout—can leave
`default_prompts_pending` true across later turns until the app is restarted. The current
specs assume oneshots always settle; they do not define hang recovery, so the UI has no
path back to a usable default list without a process restart.

## Scope

```
caps/
├── chat/
│   └── default-prompts/   (modified — settle/recover when oneshot never completes)
└── harness/
    └── warm-runtime/      (modified — oneshot timeout / queue not permanently wedged)
```

### New capabilities

- None

### Modified capabilities

- `chat/default-prompts` — Pending must end in ready (success, failure, or
  timeout/recovery). Empty input must not stay on loading forever after a turn. Later
  turns must re-arm usable defaults (oneshot list or lifecycle heuristic).

- `harness/warm-runtime` — The per-chat oneshot path must not stay wedged after a hung
  title or reply call. A bounded wait or equivalent isolation so a later oneshot on the
  same handle can still run.

### Out of scope

- Changing suggestion parse rules or the shape of the empty-input chrome (list vs loading
  indicator)

- Title-summary quality or when titles are requested

- Main-turn cancel semantics beyond what oneshot recovery needs

- New suggestion features (more replies, persistence, different models)

## Impact

```
TurnComplete ──► begin pending ──► oneshot queue ──► DefaultPromptsReady
                      │                 │                    │
                      │            (can hang)           (never arrives)
                      ▼                 ▼                    ▼
                 empty input ──► "…" stuck ──► next turn still "…"
```

- `duckchat` worker oneshot loop and Claude/Grok oneshot runtimes gain hang bounds or
  recovery so the serial queue can drain

- `duckboard` pending/generation wire-up and loading chrome must treat timeout/recovery as
  a settle (ready with effective list or heuristic fallback)

- No intentional API break for main turns; oneshot callers may see errors/timeouts where
  they previously hung forever
