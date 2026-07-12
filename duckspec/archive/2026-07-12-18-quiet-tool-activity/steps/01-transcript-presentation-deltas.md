# Transcript presentation deltas

Capture the calm secondary chrome contract on `chat/transcript` so Activity matches
Thinking and kind icons are documented before the view change.

## Tasks

- [x] 1. Add `caps/chat/transcript/doc.delta.md` describing flat Activity chrome (no
         tool-card frame/fills), shared secondary header hierarchy with User/Answer still
         primary, and thought-bubble / wrench kind icons on Thinking and Activity headers

- [x] 2. Add a thin `caps/chat/transcript/spec.delta.md` only if locking presentation
         norms (secondary chrome parity, kind icons, Activity body fade) — otherwise skip
         and keep pure paint as doc-only per design

- [x] 3. `ds format` and `ds check` the new delta paths under this change

## Outcomes

- Spec delta skipped: pure presentation paint; contract lives in the doc delta and
  existing segment/label scenarios. Manual visual check is step 03.
