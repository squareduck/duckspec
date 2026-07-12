# Resend-history hint without first-bind flash

Stop the composer “resends full history” indicator from flashing on a new chat’s first
turn; show it only when a stored agent session cannot resume for the current model
harness.

## Motivation

On a brand-new session, the first send (for example `/ds-explore`) briefly shows “resends
full history” even though nothing is being re-fed. The footer treats “no resumable agent
session yet” the same as “next send will re-send history,” so the first-bind window looks
like a real resend warning.

The honest cases are a harness/model switch (stored id is foreign to the selected harness)
and other durable “can’t resume with this id” states — not mid first bind, and not after
recovery has already cleared the id and re-sent on its own.

Why now: the false positive is visible on every new exploration and trains people to
ignore a warning that should only fire when history really will be re-fed.

## Intent

- The resend-history hint appears only when the transcript is non-empty **and** a stored
  agent session id exists **and** that id is not resumable for the effective harness
  (typically after a harness switch).

- On a new chat’s first turn — before any agent session id is bound — the hint stays
  hidden for the whole first-bind window (no multi-second flash).

- After lost-session recovery clears the stored id, the hint stays silent; recovery may
  still re-send history without advertising it in the footer.

- When the id is present and matches the harness (normal resume), the hint stays hidden as
  today.

- The rule stays a small, testable footer decision — not a redesign of resume, preamble,
  or recovery mechanics.

## Non-goals

- Changing when history is actually re-fed or how the preamble is built
- Changing lost-session recovery behavior (only whether the footer advertises it)
- Redesigning the rest of the composer footer (usage readout, model label, chrome)
- Same-harness model picks that still resume the same agent session
- New session UX beyond this indicator’s visibility
